//! The Map: the inside of the mind as a graph in 3D (the Specchio room Map).
//!
//! What the mind holds is made of Monadi; today, of 512-bit blocks generated from
//! names and composed by algebra (nothing is learned yet). The Map shows them and how
//! they refer to each other:
//!
//! - **word**: the generated vector of a word (`it.cane`);
//! - **role**: the generated vector of a position in the sentence (`ruolo.1`, …);
//!   there is no syntax yet, so the roles are positions;
//! - **binding**: role XOR word, the word in its place;
//! - **sentence**: the majority of its bindings (with an even number of words, a fixed
//!   generated vector breaks the ties);
//! - **memory**: the majority of all the sentences, what a blackboard would hold.
//!
//! Edges: `part` (what a node is made of), `answer` (asking a sentence for a role gives
//! back the nearest word, with its distance), `near` (two nodes much closer than chance:
//! 256 of 512 bits apart, ± 11.3; near means at most 211, four standard deviations).
//! Every node carries its bits, so the page can draw it as a cube of 8 × 8 × 8 cells
//! (bit order = octal address = Morton order).
//!
//! Integer arithmetic only; the layout in 3D is done by the page.

use monade_core::size::Size;
use monade_core::space::Space;
use monade_core::word::{self, Word};
use serde_json::{json, Value};

/// At most this many sentences, and words per sentence: 8, as everywhere.
pub const MAX: usize = 8;
/// Two nodes are near when at most this many bits apart (chance: 256 ± 11.3 of 512).
pub const NEAR: u64 = 211;
/// The sentences shown when nothing is asked.
pub const DEFAULT: &str = "il cane dorme\nil gatto corre\nil cane corre";

struct Node {
    id: String,
    kind: &'static str,
    label: String,
    bits: Vec<Word>,
}

/// The words of a line: letters and digits only, lower case, at most [`MAX`].
fn words(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .take(MAX)
        .collect()
}

fn xor(a: &[Word], b: &[Word]) -> Vec<Word> {
    a.iter().zip(b).map(|(x, y)| x.xor(*y)).collect()
}

fn bundle(parts: &[Vec<Word>], space: &Space) -> Vec<Word> {
    let tie = space.named("segno.pareggio", Size::BLOCK);
    let mut refs: Vec<&[Word]> = parts.iter().map(Vec::as_slice).collect();
    if refs.len().is_multiple_of(2) {
        refs.push(&tie);
    }
    let mut out = vec![Word::ZERO; Size::BLOCK.words()];
    // Same lengths by construction: majority cannot fail here.
    let _ = word::majority(&refs, &mut out);
    out
}

fn hex(bits: &[Word]) -> String {
    bits.iter().map(|w| format!("{:016x}", w.bits())).collect()
}

fn distance(a: &[Word], b: &[Word]) -> u64 {
    word::hamming(a, b).unwrap_or(u64::MAX)
}

/// The graph of `text` (one sentence per line) as JSON: `{nodes, edges, near, bits}`.
pub fn graph(space: &Space, text: &str) -> Value {
    let sentences: Vec<Vec<String>> = text
        .lines()
        .map(words)
        .filter(|w| !w.is_empty())
        .take(MAX)
        .collect();
    let v = |name: &str| space.named(name, Size::BLOCK);
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Value> = Vec::new();
    let index = |nodes: &mut Vec<Node>, id: String, kind, label: String, bits: Vec<Word>| {
        if !nodes.iter().any(|n| n.id == id) {
            nodes.push(Node {
                id: id.clone(),
                kind,
                label,
                bits,
            });
        }
        id
    };
    let mut vocabulary: Vec<String> = Vec::new();
    let mut sentence_ids = Vec::new();
    let mut sentence_bits = Vec::new();
    for (s, ws) in sentences.iter().enumerate() {
        let mut bindings = Vec::new();
        let mut binding_ids = Vec::new();
        for (p, w) in ws.iter().enumerate() {
            let wv = v(&format!("it.{w}"));
            let rv = v(&format!("ruolo.{}", p + 1));
            let wid = index(&mut nodes, format!("w:{w}"), "word", w.clone(), wv.clone());
            let rid = index(
                &mut nodes,
                format!("r:{}", p + 1),
                "role",
                format!("role {}", p + 1),
                rv.clone(),
            );
            if !vocabulary.contains(w) {
                vocabulary.push(w.clone());
            }
            let b = xor(&rv, &wv);
            let bid = index(
                &mut nodes,
                format!("b:{}:{w}", p + 1),
                "binding",
                format!("{w} @ {}", p + 1),
                b.clone(),
            );
            for to in [&wid, &rid] {
                edges.push(json!({"from": bid, "to": to, "kind": "part"}));
            }
            binding_ids.push(bid);
            bindings.push(b);
        }
        let sv = bundle(&bindings, space);
        let sid = index(
            &mut nodes,
            format!("s:{s}"),
            "sentence",
            ws.join(" "),
            sv.clone(),
        );
        for b in &binding_ids {
            edges.push(json!({"from": sid, "to": b, "kind": "part"}));
        }
        sentence_ids.push((sid, ws.len()));
        sentence_bits.push(sv);
    }
    // Asking each sentence for each role: the nearest word of the vocabulary.
    for ((sid, n), sv) in sentence_ids.iter().zip(&sentence_bits) {
        for p in 1..=*n {
            let q = xor(sv, &v(&format!("ruolo.{p}")));
            if let Some((d, w)) = vocabulary
                .iter()
                .map(|w| (distance(&q, &v(&format!("it.{w}"))), w))
                .min()
            {
                edges.push(
                    json!({"from": sid, "to": format!("w:{w}"), "kind": "answer",
                    "role": p, "distance": d}),
                );
            }
        }
    }
    if sentence_bits.len() > 1 {
        let mv = bundle(&sentence_bits, space);
        let mid = index(&mut nodes, "m".into(), "memory", "memory".into(), mv);
        for (sid, _) in &sentence_ids {
            edges.push(json!({"from": mid, "to": sid, "kind": "part"}));
        }
    }
    // Near: every pair much closer than chance, not already joined by a part.
    let mut near = Vec::new();
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            let d = distance(&nodes[i].bits, &nodes[j].bits);
            let joined = edges.iter().any(|e| {
                (e["from"] == nodes[i].id.as_str() && e["to"] == nodes[j].id.as_str())
                    || (e["from"] == nodes[j].id.as_str() && e["to"] == nodes[i].id.as_str())
            });
            if d <= NEAR && !joined {
                near.push(
                    json!({"from": nodes[i].id, "to": nodes[j].id, "kind": "near", "distance": d}),
                );
            }
        }
    }
    let list: Vec<Value> = nodes
        .iter()
        .map(|n| {
            json!({"id": n.id, "kind": n.kind, "label": n.label,
                "ones": word::popcount(&n.bits), "bits": hex(&n.bits)})
        })
        .collect();
    json!({"nodes": list, "edges": edges, "near": near, "size": Size::BLOCK.bits(),
        "near_at_most": NEAR, "sentences": sentences.len()})
}

/// Decodes a query value (`+` and `%XX`); bad escapes are kept as they are.
pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
                {
                    Ok(x) => {
                        out.push(x);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The value of `key` in the query of `url`.
pub fn query(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1.split('#').next()?;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == key).then(|| decode(v))
    })
}

/// The page of the room: a canvas, the sentences to show, a panel for one node.
pub fn page() -> String {
    PAGE.replace("{{DEFAULT}}", DEFAULT)
}

const PAGE: &str = include_str!("map.html");

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(g: &'a Value, id: &str) -> &'a Value {
        g["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap()
    }

    #[test]
    fn the_default_graph() {
        let g = graph(&Space::genesis(), DEFAULT);
        assert_eq!(g["sentences"], 3);
        // 5 words, 3 roles, 5 bindings (shared: il@1, cane@2, corre@3), 3 sentences, memory.
        let kinds = |k: &str| {
            g["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|n| n["kind"] == k)
                .count()
        };
        assert_eq!(
            (
                kinds("word"),
                kinds("role"),
                kinds("binding"),
                kinds("sentence"),
                kinds("memory")
            ),
            (5, 3, 5, 3, 1)
        );
        assert_eq!(find(&g, "w:cane")["bits"].as_str().unwrap().len(), 128);
        // Every role of every sentence is answered with the right word.
        let answers: Vec<&Value> = g["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "answer")
            .collect();
        assert_eq!(answers.len(), 9);
        for e in &answers {
            let s = find(&g, e["from"].as_str().unwrap())["label"]
                .as_str()
                .unwrap()
                .to_string();
            let p = e["role"].as_u64().unwrap() as usize;
            assert_eq!(e["to"], format!("w:{}", s.split(' ').nth(p - 1).unwrap()));
            assert!(e["distance"].as_u64().unwrap() <= NEAR);
        }
        // "il cane dorme" and "il cane corre" share two bindings: near.
        let near = g["near"].as_array().unwrap();
        assert!(near.iter().any(|e| (e["from"] == "s:0" && e["to"] == "s:2")
            || (e["from"] == "s:2" && e["to"] == "s:0")));
        // Two generated words are strangers: never near.
        assert!(!near
            .iter()
            .any(|e| e["from"].as_str().unwrap().starts_with("w:")
                && e["to"].as_str().unwrap().starts_with("w:")));
    }

    #[test]
    fn input_is_bounded_and_cleaned() {
        let text = "A, b! c d e f g h i j\n\n".repeat(12);
        let g = graph(&Space::genesis(), &text);
        assert_eq!(g["sentences"], MAX as u64);
        assert!(find(&g, "s:0")["label"] == "a b c d e f g h");
        assert_eq!(
            graph(&Space::genesis(), "")["nodes"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn query_values() {
        assert_eq!(
            query("/api/map?s=il+cane%0Adorme&x=1", "s").as_deref(),
            Some("il cane\ndorme")
        );
        assert_eq!(decode("%C3%A8%zz%4"), "è%zz%4");
        assert_eq!(query("/api/map", "s"), None);
    }
}

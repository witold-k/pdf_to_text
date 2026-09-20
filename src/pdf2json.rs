use anyhow::{Context, Result};
use roxmltree::{Document, Node};
use serde::Serialize;
use std::fmt;

const GROBID_URL: &str = "http://localhost:8070/api/processFulltextDocument";
const TEI_NS: &str = "http://www.tei-c.org/ns/1.0";

#[derive(Serialize)]
pub struct Section {
    pub heading: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct Reference {
    pub id: Option<String>,
    pub text: String,
}

#[derive(Serialize)]
pub struct Paper {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_: String,
    pub sections: Vec<Section>,
    pub references: Vec<Reference>,
}

impl Paper {
    // If you want pretty JSON:
    pub fn to_pretty_string(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

impl fmt::Display for Paper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

pub struct GrobidConverter;

impl GrobidConverter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrobidConverter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------
// 1) PDF → TEI  (sync version)
// ---------------------------------------------------------
impl GrobidConverter {
    pub fn extract_tei(&self, pdf_path: &str) -> Result<String> {
        let form = ureq::unversioned::multipart::Form::new()
            .file("input", pdf_path)
            .with_context(|| format!("Cannot open {pdf_path}"))?
            .text("consolidateHeader", "1")
            .text("teiCoordinates", "true")
            .text("generateIDs", "true")
            .text("segmentSentences", "true");

        let mut response = ureq::post(GROBID_URL)
            .send(form)
            .context("GROBID request failed")?;

        response
            .body_mut()
            .read_to_string()
            .context("failed to read GROBID response")
    }

    // ---------------------------------------------------------
    // 2) TEI → JSON
    // ---------------------------------------------------------
    pub fn tei_to_json(&self, xml: &str) -> Result<serde_json::Value> {
        let doc = Document::parse(xml).context("Failed to parse TEI XML")?;
        let root = doc.root_element();

        let title = self.find_first_text(&root, "title").unwrap_or_default();
        let authors = self.find_all(&root, "author");
        let abstract_ = self.find_first_text(&root, "abstract").unwrap_or_default();
        let sections = self.collect_sections(&root);
        let references = self.collect_references(&root);

        let paper = Paper {
            title,
            authors,
            abstract_,
            sections,
            references,
        };

        serde_json::to_value(paper).map_err(|e| e.into())
    }

    // ---------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------
    fn find_first_text(&self, root: &Node, tag: &str) -> Option<String> {
        for n in root.descendants() {
            if n.is_element()
                && n.tag_name().namespace() == Some(TEI_NS)
                && n.tag_name().name() == tag
            {
                return Some(self.normalize(&n));
            }
        }
        None
    }

    fn find_all(&self, root: &Node, tag: &str) -> Vec<String> {
        let mut out = Vec::new();
        for n in root.descendants() {
            if n.is_element()
                && n.tag_name().namespace() == Some(TEI_NS)
                && n.tag_name().name() == tag
            {
                let t = self.normalize(&n);
                if !t.is_empty() {
                    out.push(t);
                }
            }
        }
        out
    }

    fn collect_sections(&self, root: &Node) -> Vec<Section> {
        let mut out = Vec::new();

        for div in root.descendants() {
            if div.is_element()
                && div.tag_name().namespace() == Some(TEI_NS)
                && div.tag_name().name() == "div"
            {
                if !self.has_ancestor(&div, "body") {
                    continue;
                }

                let heading = div
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "head")
                    .map(|n| self.normalize(&n))
                    .unwrap_or_default();

                let mut paras = Vec::new();
                for p in div.children() {
                    if p.is_element() && p.tag_name().name() == "p" {
                        let t = self.normalize(&p);
                        if !t.is_empty() {
                            paras.push(t);
                        }
                    }
                }

                if !heading.is_empty() || !paras.is_empty() {
                    out.push(Section {
                        heading,
                        text: paras.join("\n"),
                    });
                }
            }
        }

        out
    }

    fn collect_references(&self, root: &Node) -> Vec<Reference> {
        let mut out = Vec::new();

        for bibl in root.descendants() {
            if bibl.is_element()
                && bibl.tag_name().namespace() == Some(TEI_NS)
                && bibl.tag_name().name() == "biblStruct"
            {
                let id = bibl
                    .attribute(("http://www.w3.org/XML/1998/namespace", "id"))
                    .map(|s| s.to_string());

                let text = self.normalize(&bibl);
                if !text.is_empty() {
                    out.push(Reference { id, text });
                }
            }
        }

        out
    }

    fn has_ancestor(&self, node: &Node, tag: &str) -> bool {
        let mut cur = node.parent();
        while let Some(n) = cur {
            if n.is_element() && n.tag_name().name() == tag {
                return true;
            }
            cur = n.parent();
        }
        false
    }

    fn normalize(&self, node: &Node) -> String {
        let mut s = String::new();
        for t in node.descendants().filter_map(|n| n.text()) {
            s.push_str(t);
            s.push(' ');
        }

        let mut out = s.split_whitespace().collect::<Vec<_>>().join(" ");

        // remove GROBID boilerplate
        if out.contains("GROBID - A machine learning software") {
            out = out.replace(
                "GROBID - A machine learning software for extracting information from scholarly documents",
                "",
            );
            out = out.split_whitespace().collect::<Vec<_>>().join(" ");
        }

        out
    }
}


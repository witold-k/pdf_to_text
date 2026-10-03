use crate::error::Result;
use roxmltree::{Document, Node};
use serde::Serialize;
use std::fmt;

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
    pub fn to_pretty_string(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Plain-text projection used as the canonical input for tokenization.
    pub fn to_text(&self) -> String {
        let mut parts = Vec::new();

        if !self.title.is_empty() {
            parts.push(self.title.as_str());
        }
        if !self.abstract_.is_empty() {
            parts.push(self.abstract_.as_str());
        }
        for section in &self.sections {
            if !section.heading.is_empty() {
                parts.push(section.heading.as_str());
            }
            if !section.text.is_empty() {
                parts.push(section.text.as_str());
            }
        }

        parts.join("\n\n")
    }
}

impl fmt::Display for Paper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

pub struct GrobidConverter {
    url: String,
}

impl GrobidConverter {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

// ---------------------------------------------------------
// 1) PDF → TEI  (sync version)
// ---------------------------------------------------------
impl GrobidConverter {
    pub fn extract_tei(&self, pdf_path: &str) -> Result<String> {
        let form = ureq::unversioned::multipart::Form::new()
            .file("input", pdf_path)?
            .text("consolidateHeader", "1")
            .text("teiCoordinates", "true")
            .text("generateIDs", "true")
            .text("segmentSentences", "true");

        let mut response = ureq::post(&self.url).send(form)?;

        Ok(response.body_mut().read_to_string()?)
    }

    // ---------------------------------------------------------
    // 2) TEI → simplified document model
    // ---------------------------------------------------------
    pub fn tei_to_paper(&self, xml: &str) -> Result<Paper> {
        let doc = Document::parse(xml)?;
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

        Ok(paper)
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
                let text = n.text().unwrap_or("").trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn find_all(&self, root: &Node, tag: &str) -> Vec<String> {
        root.descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().namespace() == Some(TEI_NS)
                    && n.tag_name().name() == tag
            })
            .filter_map(|n| {
                let text = n.text().unwrap_or("").trim();
                (!text.is_empty()).then(|| text.to_string())
            })
            .collect()
    }

    fn collect_sections(&self, root: &Node) -> Vec<Section> {
        let mut sections = Vec::new();

        for div in root.descendants().filter(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(TEI_NS)
                && n.tag_name().name() == "div"
        }) {
            let heading = div
                .children()
                .find(|n| {
                    n.is_element()
                        && n.tag_name().namespace() == Some(TEI_NS)
                        && n.tag_name().name() == "head"
                })
                .and_then(|n| n.text())
                .unwrap_or("")
                .trim()
                .to_string();

            let text = div
                .descendants()
                .filter(|n| {
                    n.is_element()
                        && n.tag_name().namespace() == Some(TEI_NS)
                        && n.tag_name().name() == "p"
                })
                .filter_map(|n| n.text())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            if !heading.is_empty() || !text.is_empty() {
                sections.push(Section { heading, text });
            }
        }

        sections
    }

    fn collect_references(&self, root: &Node) -> Vec<Reference> {
        root.descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().namespace() == Some(TEI_NS)
                    && n.tag_name().name() == "biblStruct"
            })
            .map(|n| Reference {
                id: n.attribute(("http://www.w3.org/XML/1998/namespace", "id"))
                    .map(str::to_string),
                text: n.text().unwrap_or("").trim().to_string(),
            })
            .collect()
    }
}

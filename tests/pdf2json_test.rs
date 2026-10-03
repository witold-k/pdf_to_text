use pdf_to_text::pdf2json::GrobidConverter;

const TEI: &str = r#"
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt>
        <title>Example title</title>
        <author>Alice Example</author>
      </titleStmt>
    </fileDesc>
    <profileDesc>
      <abstract>Example abstract</abstract>
    </profileDesc>
  </teiHeader>
  <text>
    <body>
      <div>
        <head>Introduction</head>
        <p>First paragraph.</p>
        <p>Second paragraph.</p>
      </div>
    </body>
    <back>
      <listBibl>
        <biblStruct xml:id="b0">Example reference</biblStruct>
      </listBibl>
    </back>
  </text>
</TEI>
"#;

#[test]
fn projects_tei_to_structured_json_and_canonical_text() {
    let converter = GrobidConverter::new("http://unused");
    let paper = converter.tei_to_paper(TEI).unwrap();

    let json = paper.to_pretty_string().unwrap();
    assert!(json.contains("\"title\": \"Example title\""));
    assert!(json.contains("\"authors\": ["));
    assert!(json.contains("\"references\": ["));

    assert_eq!(
        paper.to_text(),
        "Example title\n\nExample abstract\n\nIntroduction\n\nFirst paragraph.\nSecond paragraph."
    );
}

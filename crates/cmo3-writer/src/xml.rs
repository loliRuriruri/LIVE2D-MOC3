//! Typed XML writing (work order section 18).
//!
//! The writer never concatenates strings by hand: elements are built as a
//! typed tree and serialized with full escaping (`&`, `<`, `>`, `"`, `'`).
//! No DTDs, entities or external references are ever emitted (section 86).

use std::fmt::Write as _;

/// One XML element.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlElement {
    /// Element name (for example `CModelSource`).
    pub name: String,
    /// Attributes in deterministic order.
    pub attributes: Vec<(String, String)>,
    /// Children in deterministic order.
    pub children: Vec<XmlElement>,
}

impl XmlElement {
    /// Create an element with no attributes or children.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Add an attribute.
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((name.into(), value.into()));
        self
    }

    /// Add a numeric attribute.
    pub fn attr_i64(self, name: impl Into<String>, value: i64) -> Self {
        self.attr(name, value.to_string())
    }

    /// Add a child element.
    pub fn child(mut self, child: XmlElement) -> Self {
        self.children.push(child);
        self
    }

    /// Append a child element in place.
    pub fn push(&mut self, child: XmlElement) {
        self.children.push(child);
    }

    /// `xs.ref="#N"` convenience.
    pub fn attr_ref(self, name: impl Into<String>, id: usize) -> Self {
        self.attr(name, format!("#{id}"))
    }

    /// Serialize with 2-space indentation and a trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        render_element(self, 0, &mut out);
        out
    }

    fn render_into(&self, depth: usize, out: &mut String) {
        render_element(self, depth, out);
    }
}

fn render_element(element: &XmlElement, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let _ = write!(out, "{indent}<{}", element.name);
    for (name, value) in &element.attributes {
        let _ = write!(out, " {name}=\"{}\"", escape(value));
    }
    if element.children.is_empty() {
        let _ = writeln!(out, "/>");
        return;
    }
    let _ = writeln!(out, ">");
    for child in &element.children {
        child.render_into(depth + 1, out);
    }
    let _ = writeln!(out, "{indent}</{}>", element.name);
}

/// Escape XML text and attribute values (both quote kinds included).
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\r' => out.push_str("&#13;"),
            '\n' => out.push_str("&#10;"),
            '\t' => out.push_str("&#9;"),
            other if (other as u32) < 0x20 => {
                // Control characters are not representable in XML 1.0.
                out.push('\u{FFFD}');
            }
            other => out.push(other),
        }
    }
    out
}

/// `<i xs.n="name">value</i>` leaf used throughout the CMO3 schema.
pub fn int_leaf(name: &str, value: i64) -> XmlElement {
    XmlElement::new("i")
        .attr("xs.n", name)
        .attr("v", value.to_string())
}

/// `<f xs.n="name">value</f>` leaf with canonical float formatting.
pub fn float_leaf(name: &str, value: f32) -> XmlElement {
    XmlElement::new("f")
        .attr("xs.n", name)
        .attr("v", format_float(value))
}

/// `<s xs.n="name">text</s>` leaf.
pub fn string_leaf(name: &str, value: &str) -> XmlElement {
    XmlElement::new("s").attr("xs.n", name).attr("v", value)
}

/// `<b xs.n="name">true|false</b>` leaf.
pub fn bool_leaf(name: &str, value: bool) -> XmlElement {
    XmlElement::new("b")
        .attr("xs.n", name)
        .attr("v", if value { "true" } else { "false" })
}

/// `<int-array xs.n="name">...` flat integer array.
pub fn int_array(name: &str, values: &[i64]) -> XmlElement {
    let mut element = XmlElement::new("int-array").attr("xs.n", name);
    element.children.extend(
        values
            .iter()
            .map(|value| XmlElement::new("i").attr("v", value.to_string())),
    );
    element
}

/// `<float-array xs.n="name">...` flat float array.
pub fn float_array(name: &str, values: &[f32]) -> XmlElement {
    let mut element = XmlElement::new("float-array").attr("xs.n", name);
    element.children.extend(
        values
            .iter()
            .map(|value| XmlElement::new("f").attr("v", format_float(*value))),
    );
    element
}

/// Canonical float formatting: shortest representation, `f32` source values.
pub fn format_float(value: f32) -> String {
    debug_assert!(
        value.is_finite(),
        "non-finite floats must be rejected by the mapping layer before serialization"
    );
    if value == 0.0 {
        return "0.0".to_string();
    }
    let mut text = format!("{value}");
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_covers_all_five_entities() {
        assert_eq!(escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    }

    #[test]
    fn rendering_is_deterministic_and_indented() {
        let element = XmlElement::new("root")
            .attr("fileFormatVersion", "402030000")
            .child(XmlElement::new("shared"))
            .child(int_leaf("count", 3));
        let first = element.render();
        let second = element.render();
        assert_eq!(first, second);
        assert!(first.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(first.contains(r#"<root fileFormatVersion="402030000">"#));
        assert!(first.contains(r#"<i xs.n="count" v="3"/>"#));
    }

    #[test]
    fn floats_are_canonical() {
        assert_eq!(format_float(0.0), "0.0");
        assert_eq!(format_float(1.0), "1.0");
        assert_eq!(format_float(-0.5), "-0.5");
    }
}

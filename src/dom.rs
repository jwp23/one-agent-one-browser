#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub root: Element,
}

impl Document {
    pub fn render_root(&self) -> &Element {
        self.find_first_element_by_name("html")
            .or_else(|| self.find_first_element_by_name("body"))
            .unwrap_or(&self.root)
    }

    pub fn find_first_element_by_name(&self, name: &str) -> Option<&Element> {
        self.root.find_first_element_by_name(name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attributes {
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub style: Option<String>,
    others: Vec<(String, String)>,
}

impl Attributes {
    pub fn insert(&mut self, name: String, value: String) {
        match name.as_str() {
            "id" => self.id = Some(value),
            "class" => {
                self.classes.clear();
                self.classes.extend(value.split_whitespace().map(str::to_owned));
            }
            "style" => self.style = Some(value),
            _ => self.others.push((name, value)),
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        match name {
            "id" => self.id.as_deref(),
            "style" => self.style.as_deref(),
            "class" => None,
            _ => self
                .others
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str()),
        }
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == class)
    }

    pub fn to_serialized_pairs(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(id) = &self.id {
            out.push(("id".to_owned(), id.clone()));
        }
        if !self.classes.is_empty() {
            out.push(("class".to_owned(), self.classes.join(" ")));
        }
        if let Some(style) = &self.style {
            out.push(("style".to_owned(), style.clone()));
        }
        out.extend(self.others.iter().cloned());
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Attributes,
    pub children: Vec<Node>,
}

impl Element {
    pub fn find_first_element_by_name(&self, name: &str) -> Option<&Element> {
        fn walk<'a>(node: &'a Node, name: &str) -> Option<&'a Element> {
            match node {
                Node::Element(el) => {
                    if el.name == name {
                        return Some(el);
                    }
                    for child in &el.children {
                        if let Some(found) = walk(child, name) {
                            return Some(found);
                        }
                    }
                    None
                }
                Node::Text(_) => None,
            }
        }

        for child in &self.children {
            if let Some(found) = walk(child, name) {
                return Some(found);
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

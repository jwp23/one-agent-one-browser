#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub root: Element,
}

impl Document {
    pub fn render_root(&self) -> &Element {
        self.find_first_element_by_name("body")
            .unwrap_or(&self.root)
    }

    fn find_first_element_by_name(&self, name: &str) -> Option<&Element> {
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

        for child in &self.root.children {
            if let Some(found) = walk(child, name) {
                return Some(found);
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}


pub type ElementId = u64;

#[derive(Clone, Debug)]
pub struct Document {
    pub root: Element,
    next_element_id: ElementId,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl Eq for Document {}

#[derive(Clone)]
struct SelectorSet {
    selectors: Vec<crate::css::Selector>,
}

impl Document {
    pub fn new(root: Element, next_element_id: ElementId) -> Self {
        Self {
            root,
            next_element_id,
        }
    }

    pub fn render_root(&self) -> &Element {
        self.find_first_element_by_name("html")
            .or_else(|| self.find_first_element_by_name("body"))
            .unwrap_or(&self.root)
    }

    pub fn find_first_element_by_name(&self, name: &str) -> Option<&Element> {
        self.root.find_first_element_by_name(name)
    }

    pub fn find_first_element_by_name_mut(&mut self, name: &str) -> Option<&mut Element> {
        self.root.find_first_element_by_name_mut(name)
    }

    pub fn find_first_element_by_id(&self, id: &str) -> Option<&Element> {
        self.root.find_first_element_by_id(id)
    }

    pub fn find_first_element_by_id_mut(&mut self, id: &str) -> Option<&mut Element> {
        self.root.find_first_element_by_id_mut(id)
    }

    pub fn find_element_by_node_id(&self, node_id: ElementId) -> Option<&Element> {
        self.root.find_element_by_node_id(node_id)
    }

    pub fn find_element_by_node_id_mut(&mut self, node_id: ElementId) -> Option<&mut Element> {
        self.root.find_element_by_node_id_mut(node_id)
    }

    pub fn query_selector(&self, selector: &str) -> Option<&Element> {
        let selector_set = SelectorSet::parse(selector)?;
        self.query_selector_from_set(&selector_set)
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<&Element> {
        let Some(selector_set) = SelectorSet::parse(selector) else {
            return Vec::new();
        };
        self.query_selector_all_from_set(&selector_set)
    }

    pub fn query_selector_id(&self, selector: &str) -> Option<ElementId> {
        self.query_selector(selector).map(|element| element.node_id)
    }

    pub fn query_selector_all_ids(&self, selector: &str) -> Vec<ElementId> {
        self.query_selector_all(selector)
            .into_iter()
            .map(|element| element.node_id)
            .collect()
    }

    pub fn parent_element_of(&self, node_id: ElementId) -> Option<&Element> {
        if self.root.node_id == node_id {
            return None;
        }
        self.root.find_parent_of(node_id)
    }

    pub fn append_child_to(&mut self, parent_id: ElementId, mut child: Node) -> bool {
        self.assign_node_ids(&mut child);
        let Some(parent) = self.find_element_by_node_id_mut(parent_id) else {
            return false;
        };
        parent.children.push(child);
        true
    }

    pub fn prepend_child_to(&mut self, parent_id: ElementId, mut child: Node) -> bool {
        self.assign_node_ids(&mut child);
        let Some(parent) = self.find_element_by_node_id_mut(parent_id) else {
            return false;
        };
        parent.children.insert(0, child);
        true
    }

    pub fn replace_element_with(&mut self, target_id: ElementId, mut replacement: Element) -> bool {
        if self.root.node_id == target_id {
            self.assign_element_ids(&mut replacement);
            self.root = replacement;
            return true;
        }

        self.assign_element_ids(&mut replacement);
        self.root
            .replace_child_element(target_id, replacement)
            .is_some()
    }

    pub fn remove_element(&mut self, target_id: ElementId) -> bool {
        if self.root.node_id == target_id {
            return false;
        }
        self.root.remove_child_element(target_id).is_some()
    }

    pub fn allocate_element_id(&mut self) -> ElementId {
        let id = self.next_element_id.max(1);
        self.next_element_id = id.saturating_add(1);
        id
    }

    pub fn assign_element_ids(&mut self, element: &mut Element) {
        if element.node_id == 0 {
            element.node_id = self.allocate_element_id();
        }
        for child in &mut element.children {
            self.assign_node_ids(child);
        }
    }

    pub fn assign_node_ids(&mut self, node: &mut Node) {
        if let Node::Element(element) = node {
            self.assign_element_ids(element);
        }
    }

    fn query_selector_from_set<'a>(&'a self, selector_set: &SelectorSet) -> Option<&'a Element> {
        for child in &self.root.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(found) = element.query_selector_from_set(selector_set, &[]) {
                return Some(found);
            }
        }
        None
    }

    fn query_selector_all_from_set<'a>(&'a self, selector_set: &SelectorSet) -> Vec<&'a Element> {
        let mut out = Vec::new();
        for child in &self.root.children {
            let Node::Element(element) = child else {
                continue;
            };
            element.collect_query_matches(selector_set, &[], &mut out);
        }
        out
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
                self.classes
                    .extend(value.split_whitespace().map(str::to_owned));
            }
            "style" => self.style = Some(value),
            _ => {
                if let Some((_, existing)) = self.others.iter_mut().find(|(key, _)| key == &name) {
                    *existing = value;
                } else {
                    self.others.push((name, value));
                }
            }
        }
    }

    pub fn remove(&mut self, name: &str) {
        match name {
            "id" => self.id = None,
            "class" => self.classes.clear(),
            "style" => self.style = None,
            _ => self.others.retain(|(key, _)| key != name),
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

#[derive(Clone, Debug)]
pub struct Element {
    pub node_id: ElementId,
    pub name: String,
    pub attributes: Attributes,
    pub children: Vec<Node>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.attributes == other.attributes
            && self.children == other.children
    }
}

impl Eq for Element {}

impl Element {
    pub fn new(name: impl Into<String>, attributes: Attributes, children: Vec<Node>) -> Self {
        Self {
            node_id: 0,
            name: name.into(),
            attributes,
            children,
        }
    }

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

    pub fn find_first_element_by_name_mut(&mut self, name: &str) -> Option<&mut Element> {
        for child in &mut self.children {
            let Node::Element(el) = child else {
                continue;
            };

            if el.name == name {
                return Some(el);
            }
            if let Some(found) = el.find_first_element_by_name_mut(name) {
                return Some(found);
            }
        }

        None
    }

    pub fn find_first_element_by_id(&self, id: &str) -> Option<&Element> {
        if self.attributes.id.as_deref() == Some(id) {
            return Some(self);
        }

        for child in &self.children {
            let Node::Element(el) = child else {
                continue;
            };
            if let Some(found) = el.find_first_element_by_id(id) {
                return Some(found);
            }
        }

        None
    }

    pub fn find_first_element_by_id_mut(&mut self, id: &str) -> Option<&mut Element> {
        if self.attributes.id.as_deref() == Some(id) {
            return Some(self);
        }

        for child in &mut self.children {
            let Node::Element(el) = child else {
                continue;
            };
            if let Some(found) = el.find_first_element_by_id_mut(id) {
                return Some(found);
            }
        }

        None
    }

    pub fn set_text_content(&mut self, text: String) {
        self.children.clear();
        self.children.push(Node::Text(text));
    }

    pub fn first_element_child(&self) -> Option<&Element> {
        self.children.iter().find_map(|child| match child {
            Node::Element(element) => Some(element),
            Node::Text(_) => None,
        })
    }

    pub fn last_element_child(&self) -> Option<&Element> {
        self.children.iter().rev().find_map(|child| match child {
            Node::Element(element) => Some(element),
            Node::Text(_) => None,
        })
    }

    pub fn query_selector(&self, selector: &str) -> Option<&Element> {
        let selector_set = SelectorSet::parse(selector)?;
        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(found) = element.query_selector_from_set(&selector_set, &[]) {
                return Some(found);
            }
        }
        None
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<&Element> {
        let Some(selector_set) = SelectorSet::parse(selector) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            element.collect_query_matches(&selector_set, &[], &mut out);
        }
        out
    }

    fn find_element_by_node_id(&self, node_id: ElementId) -> Option<&Element> {
        if self.node_id == node_id {
            return Some(self);
        }

        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(found) = element.find_element_by_node_id(node_id) {
                return Some(found);
            }
        }

        None
    }

    fn find_element_by_node_id_mut(&mut self, node_id: ElementId) -> Option<&mut Element> {
        if self.node_id == node_id {
            return Some(self);
        }

        for child in &mut self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(found) = element.find_element_by_node_id_mut(node_id) {
                return Some(found);
            }
        }

        None
    }

    fn find_parent_of(&self, node_id: ElementId) -> Option<&Element> {
        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if element.node_id == node_id {
                return Some(self);
            }
            if let Some(found) = element.find_parent_of(node_id) {
                return Some(found);
            }
        }
        None
    }

    fn replace_child_element(
        &mut self,
        node_id: ElementId,
        replacement: Element,
    ) -> Option<Element> {
        for child in &mut self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if element.node_id == node_id {
                let old = std::mem::replace(element, replacement);
                return Some(old);
            }
            if let Some(old) = element.replace_child_element(node_id, replacement.clone()) {
                return Some(old);
            }
        }
        None
    }

    fn remove_child_element(&mut self, node_id: ElementId) -> Option<Element> {
        let index = self.children.iter().position(|child| match child {
            Node::Element(element) => element.node_id == node_id,
            Node::Text(_) => false,
        });
        if let Some(index) = index {
            return match self.children.remove(index) {
                Node::Element(element) => Some(element),
                Node::Text(_) => None,
            };
        }

        for child in &mut self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(removed) = element.remove_child_element(node_id) {
                return Some(removed);
            }
        }

        None
    }

    fn query_selector_from_set<'a>(
        &'a self,
        selector_set: &SelectorSet,
        ancestors: &[&'a Element],
    ) -> Option<&'a Element> {
        if selector_set.matches(self, ancestors) {
            return Some(self);
        }

        let mut next_ancestors = Vec::with_capacity(ancestors.len().saturating_add(1));
        next_ancestors.extend_from_slice(ancestors);
        next_ancestors.push(self);

        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            if let Some(found) = element.query_selector_from_set(selector_set, &next_ancestors) {
                return Some(found);
            }
        }

        None
    }

    fn collect_query_matches<'a>(
        &'a self,
        selector_set: &SelectorSet,
        ancestors: &[&'a Element],
        out: &mut Vec<&'a Element>,
    ) {
        if selector_set.matches(self, ancestors) {
            out.push(self);
        }

        let mut next_ancestors = Vec::with_capacity(ancestors.len().saturating_add(1));
        next_ancestors.extend_from_slice(ancestors);
        next_ancestors.push(self);

        for child in &self.children {
            let Node::Element(element) = child else {
                continue;
            };
            element.collect_query_matches(selector_set, &next_ancestors, out);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

impl SelectorSet {
    fn parse(input: &str) -> Option<Self> {
        let selectors = crate::css::parse_selectors(input);
        (!selectors.is_empty()).then_some(Self { selectors })
    }

    fn matches(&self, element: &Element, ancestors: &[&Element]) -> bool {
        self.selectors.iter().any(|selector| {
            crate::style::selectors::selector_matches(selector, element, ancestors)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_selector_reuses_css_selector_matching() {
        let document = crate::html::parse_document(
            r#"
            <div class="mw-header">
              <button class="search-toggle" id="search-toggle"></button>
            </div>
            "#,
        );

        let button = document
            .query_selector(".mw-header .search-toggle")
            .expect("missing button");

        assert_eq!(button.attributes.id.as_deref(), Some("search-toggle"));
    }

    #[test]
    fn query_selector_all_returns_all_matching_elements_in_document_order() {
        let document = crate::html::parse_document(
            r#"
            <ul>
              <li class="item" id="a"></li>
              <li class="item" id="b"></li>
              <li id="c"></li>
            </ul>
            "#,
        );

        let ids: Vec<_> = document
            .query_selector_all(".item")
            .into_iter()
            .map(|element| element.attributes.id.as_deref().unwrap_or(""))
            .collect();

        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn document_mutations_assign_node_ids_to_inserted_subtrees() {
        let mut document = crate::html::parse_document(r#"<div id="host"></div>"#);
        let host_id = document
            .find_first_element_by_id("host")
            .expect("missing host")
            .node_id;

        let child = Element::new(
            "section",
            {
                let mut attributes = Attributes::default();
                attributes.insert("id".to_owned(), "child".to_owned());
                attributes
            },
            vec![Node::Element(Element::new("span", Attributes::default(), Vec::new()))],
        );

        assert!(document.append_child_to(host_id, Node::Element(child)));

        let child = document
            .find_first_element_by_id("child")
            .expect("missing inserted child");
        assert!(child.node_id > 0);
        assert!(child.first_element_child().is_some_and(|element| element.node_id > 0));
    }

    #[test]
    fn replace_and_remove_element_operate_by_node_id() {
        let mut document =
            crate::html::parse_document(r#"<div><span id="old"></span><span id="keep"></span></div>"#);
        let old_id = document
            .find_first_element_by_id("old")
            .expect("missing old element")
            .node_id;

        let replacement = Element::new(
            "span",
            {
                let mut attributes = Attributes::default();
                attributes.insert("id".to_owned(), "new".to_owned());
                attributes
            },
            Vec::new(),
        );

        assert!(document.replace_element_with(old_id, replacement));
        assert!(document.find_first_element_by_id("old").is_none());

        let new_id = document
            .find_first_element_by_id("new")
            .expect("missing replacement")
            .node_id;
        assert!(document.remove_element(new_id));
        assert!(document.find_first_element_by_id("new").is_none());
        assert!(document.find_first_element_by_id("keep").is_some());
    }
}

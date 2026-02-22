use crate::dom::{Element, Node};

pub(super) fn serialize_element_xml(element: &Element) -> String {
    let mut out = String::new();
    let svg_mode = element.name == "svg";
    write_element_xml(element, &mut out, svg_mode);
    out
}

fn write_element_xml(element: &Element, out: &mut String, svg_mode: bool) {
    let tag_name = if svg_mode {
        svg_adjust_tag_name(&element.name)
    } else {
        std::borrow::Cow::Borrowed(element.name.as_str())
    };

    out.push('<');
    out.push_str(tag_name.as_ref());

    for (name, value) in element.attributes.to_serialized_pairs() {
        out.push(' ');
        let name = if svg_mode {
            svg_adjust_attr_name(&name)
        } else {
            std::borrow::Cow::Borrowed(name.as_str())
        };
        out.push_str(name.as_ref());
        out.push_str("=\"");
        write_xml_escaped(&value, out, true);
        out.push('"');
    }

    out.push('>');
    for child in &element.children {
        match child {
            Node::Text(text) => write_xml_escaped(text, out, false),
            Node::Element(child) => write_element_xml(child, out, svg_mode),
        }
    }
    out.push_str("</");
    out.push_str(tag_name.as_ref());
    out.push('>');
}

fn write_xml_escaped(value: &str, out: &mut String, for_attribute: bool) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if for_attribute => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

fn svg_adjust_tag_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name {
        "altglyph" => std::borrow::Cow::Borrowed("altGlyph"),
        "altglyphdef" => std::borrow::Cow::Borrowed("altGlyphDef"),
        "altglyphitem" => std::borrow::Cow::Borrowed("altGlyphItem"),
        "animatecolor" => std::borrow::Cow::Borrowed("animateColor"),
        "animatemotion" => std::borrow::Cow::Borrowed("animateMotion"),
        "animatetransform" => std::borrow::Cow::Borrowed("animateTransform"),
        "clippath" => std::borrow::Cow::Borrowed("clipPath"),
        "feblend" => std::borrow::Cow::Borrowed("feBlend"),
        "fecolormatrix" => std::borrow::Cow::Borrowed("feColorMatrix"),
        "fecomponenttransfer" => std::borrow::Cow::Borrowed("feComponentTransfer"),
        "fecomposite" => std::borrow::Cow::Borrowed("feComposite"),
        "feconvolvematrix" => std::borrow::Cow::Borrowed("feConvolveMatrix"),
        "fediffuselighting" => std::borrow::Cow::Borrowed("feDiffuseLighting"),
        "fedisplacementmap" => std::borrow::Cow::Borrowed("feDisplacementMap"),
        "fedistantlight" => std::borrow::Cow::Borrowed("feDistantLight"),
        "fedropshadow" => std::borrow::Cow::Borrowed("feDropShadow"),
        "feflood" => std::borrow::Cow::Borrowed("feFlood"),
        "fefunca" => std::borrow::Cow::Borrowed("feFuncA"),
        "fefuncb" => std::borrow::Cow::Borrowed("feFuncB"),
        "fefuncg" => std::borrow::Cow::Borrowed("feFuncG"),
        "fefuncr" => std::borrow::Cow::Borrowed("feFuncR"),
        "fegaussianblur" => std::borrow::Cow::Borrowed("feGaussianBlur"),
        "feimage" => std::borrow::Cow::Borrowed("feImage"),
        "femerge" => std::borrow::Cow::Borrowed("feMerge"),
        "femergenode" => std::borrow::Cow::Borrowed("feMergeNode"),
        "femorphology" => std::borrow::Cow::Borrowed("feMorphology"),
        "feoffset" => std::borrow::Cow::Borrowed("feOffset"),
        "fepointlight" => std::borrow::Cow::Borrowed("fePointLight"),
        "fespecularlighting" => std::borrow::Cow::Borrowed("feSpecularLighting"),
        "fespotlight" => std::borrow::Cow::Borrowed("feSpotLight"),
        "fetile" => std::borrow::Cow::Borrowed("feTile"),
        "feturbulence" => std::borrow::Cow::Borrowed("feTurbulence"),
        "foreignobject" => std::borrow::Cow::Borrowed("foreignObject"),
        "glyphref" => std::borrow::Cow::Borrowed("glyphRef"),
        "lineargradient" => std::borrow::Cow::Borrowed("linearGradient"),
        "radialgradient" => std::borrow::Cow::Borrowed("radialGradient"),
        "textpath" => std::borrow::Cow::Borrowed("textPath"),
        _ => std::borrow::Cow::Borrowed(name),
    }
}

fn svg_adjust_attr_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name {
        "attributename" => std::borrow::Cow::Borrowed("attributeName"),
        "attributetype" => std::borrow::Cow::Borrowed("attributeType"),
        "basefrequency" => std::borrow::Cow::Borrowed("baseFrequency"),
        "baseprofile" => std::borrow::Cow::Borrowed("baseProfile"),
        "clippathunits" => std::borrow::Cow::Borrowed("clipPathUnits"),
        "diffuseconstant" => std::borrow::Cow::Borrowed("diffuseConstant"),
        "edgemode" => std::borrow::Cow::Borrowed("edgeMode"),
        "filterunits" => std::borrow::Cow::Borrowed("filterUnits"),
        "glyphref" => std::borrow::Cow::Borrowed("glyphRef"),
        "gradienttransform" => std::borrow::Cow::Borrowed("gradientTransform"),
        "gradientunits" => std::borrow::Cow::Borrowed("gradientUnits"),
        "kernelmatrix" => std::borrow::Cow::Borrowed("kernelMatrix"),
        "kernelunitlength" => std::borrow::Cow::Borrowed("kernelUnitLength"),
        "keypoints" => std::borrow::Cow::Borrowed("keyPoints"),
        "keysplines" => std::borrow::Cow::Borrowed("keySplines"),
        "keytimes" => std::borrow::Cow::Borrowed("keyTimes"),
        "lengthadjust" => std::borrow::Cow::Borrowed("lengthAdjust"),
        "limitingconeangle" => std::borrow::Cow::Borrowed("limitingConeAngle"),
        "markerheight" => std::borrow::Cow::Borrowed("markerHeight"),
        "markerunits" => std::borrow::Cow::Borrowed("markerUnits"),
        "markerwidth" => std::borrow::Cow::Borrowed("markerWidth"),
        "maskcontentunits" => std::borrow::Cow::Borrowed("maskContentUnits"),
        "maskunits" => std::borrow::Cow::Borrowed("maskUnits"),
        "numoctaves" => std::borrow::Cow::Borrowed("numOctaves"),
        "pathlength" => std::borrow::Cow::Borrowed("pathLength"),
        "patterncontentunits" => std::borrow::Cow::Borrowed("patternContentUnits"),
        "patterntransform" => std::borrow::Cow::Borrowed("patternTransform"),
        "patternunits" => std::borrow::Cow::Borrowed("patternUnits"),
        "pointsatx" => std::borrow::Cow::Borrowed("pointsAtX"),
        "pointsaty" => std::borrow::Cow::Borrowed("pointsAtY"),
        "pointsatz" => std::borrow::Cow::Borrowed("pointsAtZ"),
        "preservealpha" => std::borrow::Cow::Borrowed("preserveAlpha"),
        "preserveaspectratio" => std::borrow::Cow::Borrowed("preserveAspectRatio"),
        "primitiveunits" => std::borrow::Cow::Borrowed("primitiveUnits"),
        "refx" => std::borrow::Cow::Borrowed("refX"),
        "refy" => std::borrow::Cow::Borrowed("refY"),
        "repeatcount" => std::borrow::Cow::Borrowed("repeatCount"),
        "repeatdur" => std::borrow::Cow::Borrowed("repeatDur"),
        "requiredextensions" => std::borrow::Cow::Borrowed("requiredExtensions"),
        "requiredfeatures" => std::borrow::Cow::Borrowed("requiredFeatures"),
        "specularconstant" => std::borrow::Cow::Borrowed("specularConstant"),
        "specularexponent" => std::borrow::Cow::Borrowed("specularExponent"),
        "spreadmethod" => std::borrow::Cow::Borrowed("spreadMethod"),
        "startoffset" => std::borrow::Cow::Borrowed("startOffset"),
        "stddeviation" => std::borrow::Cow::Borrowed("stdDeviation"),
        "stitchtiles" => std::borrow::Cow::Borrowed("stitchTiles"),
        "surfacescale" => std::borrow::Cow::Borrowed("surfaceScale"),
        "systemlanguage" => std::borrow::Cow::Borrowed("systemLanguage"),
        "tablevalues" => std::borrow::Cow::Borrowed("tableValues"),
        "targetx" => std::borrow::Cow::Borrowed("targetX"),
        "targety" => std::borrow::Cow::Borrowed("targetY"),
        "textlength" => std::borrow::Cow::Borrowed("textLength"),
        "viewbox" => std::borrow::Cow::Borrowed("viewBox"),
        "xchannelselector" => std::borrow::Cow::Borrowed("xChannelSelector"),
        "ychannelselector" => std::borrow::Cow::Borrowed("yChannelSelector"),
        "zoomandpan" => std::borrow::Cow::Borrowed("zoomAndPan"),
        _ => std::borrow::Cow::Borrowed(name),
    }
}

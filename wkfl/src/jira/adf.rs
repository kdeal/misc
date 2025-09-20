use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root ADF document structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Document {
    #[serde(rename = "type")]
    pub document_type: String, // Should always be "doc"
    pub version: u32, // ADF version, currently 1
    pub content: Vec<Node>,
}

/// ADF Node - represents any content node in the document
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Node {
    // Block nodes
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },
    #[serde(rename = "heading")]
    Heading {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        attrs: HeadingAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },
    #[serde(rename = "blockquote")]
    Blockquote {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
        content: Vec<Node>,
    },
    #[serde(rename = "codeBlock")]
    CodeBlock {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<CodeBlockAttrs>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },
    #[serde(rename = "bulletList")]
    BulletList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
        content: Vec<Node>,
    },
    #[serde(rename = "orderedList")]
    OrderedList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<OrderedListAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "listItem")]
    ListItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
        content: Vec<Node>,
    },
    #[serde(rename = "panel")]
    Panel {
        attrs: PanelAttrs,
        content: Vec<Node>,
    },
    #[serde(rename = "rule")]
    Rule {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
    },
    #[serde(rename = "table")]
    Table {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "tableRow")]
    TableRow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
        content: Vec<Node>,
    },
    #[serde(rename = "tableCell")]
    TableCell {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableCellAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "tableHeader")]
    TableHeader {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableCellAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "mediaGroup")]
    MediaGroup { content: Vec<Node> },
    #[serde(rename = "mediaSingle")]
    MediaSingle {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<MediaSingleAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "media")]
    Media {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        attrs: MediaAttrs,
    },
    #[serde(rename = "expand")]
    Expand {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<ExpandAttrs>,
        content: Vec<Node>,
    },
    #[serde(rename = "nestedExpand")]
    NestedExpand {
        attrs: ExpandAttrs,
        content: Vec<Node>,
    },

    // Inline nodes
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
    },
    #[serde(rename = "hardBreak")]
    HardBreak {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<HashMap<String, serde_json::Value>>,
    },
    #[serde(rename = "mention")]
    Mention { attrs: MentionAttrs },
    #[serde(rename = "emoji")]
    Emoji { attrs: EmojiAttrs },
    #[serde(rename = "date")]
    Date { attrs: DateAttrs },
    #[serde(rename = "status")]
    Status { attrs: StatusAttrs },
    #[serde(rename = "inlineCard")]
    InlineCard { attrs: InlineCardAttrs },
    #[serde(rename = "blockCard")]
    BlockCard { attrs: BlockCardAttrs },
    #[serde(rename = "embedCard")]
    EmbedCard { attrs: EmbedCardAttrs },

    // Task and decision lists
    #[serde(rename = "taskList")]
    TaskList {
        attrs: TaskListAttrs,
        content: Vec<Node>,
    },
    #[serde(rename = "taskItem")]
    TaskItem {
        attrs: TaskItemAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },
    #[serde(rename = "decisionList")]
    DecisionList {
        attrs: DecisionListAttrs,
        content: Vec<Node>,
    },
    #[serde(rename = "decisionItem")]
    DecisionItem {
        attrs: DecisionItemAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    // Extensions
    #[serde(rename = "extension")]
    Extension {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        attrs: ExtensionAttrs,
    },
    #[serde(rename = "bodiedExtension")]
    BodiedExtension {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        attrs: ExtensionAttrs,
        content: Vec<Node>,
    },
    #[serde(rename = "inlineExtension")]
    InlineExtension {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
        attrs: ExtensionAttrs,
    },

    // Fallback for unknown node types
    #[serde(other)]
    Unknown,
}

/// Text formatting marks
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Mark {
    #[serde(rename = "strong")]
    Strong,
    #[serde(rename = "em")]
    Em,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "strike")]
    Strike,
    #[serde(rename = "underline")]
    Underline,
    #[serde(rename = "subsup")]
    SubSup { attrs: SubSupAttrs },
    #[serde(rename = "textColor")]
    TextColor { attrs: TextColorAttrs },
    #[serde(rename = "backgroundColor")]
    BackgroundColor { attrs: BackgroundColorAttrs },
    #[serde(rename = "link")]
    Link { attrs: LinkAttrs },
    #[serde(rename = "alignment")]
    Alignment { attrs: AlignmentAttrs },
    #[serde(rename = "indentation")]
    Indentation { attrs: IndentationAttrs },
    #[serde(rename = "border")]
    Border { attrs: BorderAttrs },
    #[serde(rename = "annotation")]
    Annotation { attrs: AnnotationAttrs },
    #[serde(rename = "breakout")]
    Breakout { attrs: BreakoutAttrs },
    #[serde(rename = "dataConsumer")]
    DataConsumer { attrs: DataConsumerAttrs },
    #[serde(rename = "fragment")]
    Fragment { attrs: FragmentAttrs },
    #[serde(other)]
    Unknown,
}

// Attribute structs for different node types
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeadingAttrs {
    pub level: u32, // 1-6
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CodeBlockAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "uniqueId", skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OrderedListAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PanelAttrs {
    #[serde(rename = "panelType")]
    pub panel_type: PanelType,
    #[serde(rename = "panelIcon", skip_serializing_if = "Option::is_none")]
    pub panel_icon: Option<String>,
    #[serde(rename = "panelIconId", skip_serializing_if = "Option::is_none")]
    pub panel_icon_id: Option<String>,
    #[serde(rename = "panelIconText", skip_serializing_if = "Option::is_none")]
    pub panel_icon_text: Option<String>,
    #[serde(rename = "panelColor", skip_serializing_if = "Option::is_none")]
    pub panel_color: Option<String>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PanelType {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "tip")]
    Tip,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TableAttrs {
    #[serde(rename = "displayMode", skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<TableDisplayMode>,
    #[serde(
        rename = "isNumberColumnEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_number_column_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<TableLayout>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TableDisplayMode {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fixed")]
    Fixed,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TableLayout {
    #[serde(rename = "wide")]
    Wide,
    #[serde(rename = "full-width")]
    FullWidth,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "align-end")]
    AlignEnd,
    #[serde(rename = "align-start")]
    AlignStart,
    #[serde(rename = "default")]
    Default,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TableCellAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colspan: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rowspan: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colwidth: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaSingleAttrs {
    pub layout: MediaLayout,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(rename = "widthType", skip_serializing_if = "Option::is_none")]
    pub width_type: Option<MediaWidthType>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MediaLayout {
    #[serde(rename = "wide")]
    Wide,
    #[serde(rename = "full-width")]
    FullWidth,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "wrap-right")]
    WrapRight,
    #[serde(rename = "wrap-left")]
    WrapLeft,
    #[serde(rename = "align-end")]
    AlignEnd,
    #[serde(rename = "align-start")]
    AlignStart,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MediaWidthType {
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "pixel")]
    Pixel,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum MediaAttrs {
    File {
        #[serde(rename = "type")]
        media_type: String, // "file" or "link"
        id: String,
        collection: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<f64>,
        #[serde(rename = "occurrenceKey", skip_serializing_if = "Option::is_none")]
        occurrence_key: Option<String>,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
    External {
        #[serde(rename = "type")]
        media_type: String, // "external"
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<f64>,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExpandAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MentionAttrs {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "accessLevel", skip_serializing_if = "Option::is_none")]
    pub access_level: Option<String>,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<UserType>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum UserType {
    #[serde(rename = "DEFAULT")]
    Default,
    #[serde(rename = "SPECIAL")]
    Special,
    #[serde(rename = "APP")]
    App,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmojiAttrs {
    #[serde(rename = "shortName")]
    pub short_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DateAttrs {
    pub timestamp: String,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatusAttrs {
    pub text: String,
    pub color: StatusColor,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum StatusColor {
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum InlineCardAttrs {
    Url {
        url: String,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
    Data {
        data: serde_json::Value,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum BlockCardAttrs {
    Datasource {
        datasource: DatasourceAttrs,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<MediaLayout>,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
    Url {
        url: String,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
    Data {
        data: serde_json::Value,
        #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
        local_id: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatasourceAttrs {
    pub id: String,
    pub parameters: serde_json::Value,
    pub views: Vec<DatasourceView>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatasourceView {
    #[serde(rename = "type")]
    pub view_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmbedCardAttrs {
    pub url: String,
    pub layout: MediaLayout,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(rename = "originalHeight", skip_serializing_if = "Option::is_none")]
    pub original_height: Option<f64>,
    #[serde(rename = "originalWidth", skip_serializing_if = "Option::is_none")]
    pub original_width: Option<f64>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskListAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskItemAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
    pub state: TaskState,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TaskState {
    #[serde(rename = "TODO")]
    Todo,
    #[serde(rename = "DONE")]
    Done,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DecisionListAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DecisionItemAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExtensionAttrs {
    #[serde(rename = "extensionKey")]
    pub extension_key: String,
    #[serde(rename = "extensionType")]
    pub extension_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<ExtensionLayout>,
    #[serde(rename = "localId", skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ExtensionLayout {
    #[serde(rename = "wide")]
    Wide,
    #[serde(rename = "full-width")]
    FullWidth,
    #[serde(rename = "default")]
    Default,
}

// Mark attribute structs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubSupAttrs {
    #[serde(rename = "type")]
    pub sub_sup_type: SubSupType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SubSupType {
    #[serde(rename = "sub")]
    Sub,
    #[serde(rename = "sup")]
    Sup,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextColorAttrs {
    pub color: String, // Hex color like "#97a0af"
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BackgroundColorAttrs {
    pub color: String, // Hex color like "#97a0af"
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LinkAttrs {
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(rename = "occurrenceKey", skip_serializing_if = "Option::is_none")]
    pub occurrence_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlignmentAttrs {
    pub align: AlignmentType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AlignmentType {
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "end")]
    End,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IndentationAttrs {
    pub level: u32, // 1-6
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BorderAttrs {
    pub size: f64,     // 1-3
    pub color: String, // Hex color
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnnotationAttrs {
    pub id: String,
    #[serde(rename = "annotationType")]
    pub annotation_type: AnnotationType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AnnotationType {
    #[serde(rename = "inlineComment")]
    InlineComment,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BreakoutAttrs {
    pub mode: BreakoutMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BreakoutMode {
    #[serde(rename = "wide")]
    Wide,
    #[serde(rename = "full-width")]
    FullWidth,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataConsumerAttrs {
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FragmentAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Document {
    /// Extract markdown from ADF (Atlassian Document Format)
    pub fn to_markdown(&self) -> String {
        extract_markdown_from_nodes(&self.content)
    }
}

/// Extract markdown from ADF nodes
pub fn extract_markdown_from_nodes(nodes: &[Node]) -> String {
    let mut text = String::new();
    let mut stack = Vec::new();

    // Initialize stack with root content items
    for node in nodes.iter().rev() {
        stack.push(node);
    }

    while let Some(node) = stack.pop() {
        match node {
            Node::Text {
                text: node_text,
                marks,
            } => {
                let formatted_text = apply_text_marks(node_text, marks);
                text.push_str(&formatted_text);
            }
            Node::HardBreak { .. } => {
                text.push('\n');
            }
            Node::Paragraph { content, .. } => {
                // Add a HardBreak to add newline after processing children
                stack.push(&Node::HardBreak { attrs: None });
                // Add children in reverse order so they're processed in correct order
                for child in content.iter().rev() {
                    stack.push(child);
                }
            }
            Node::Heading { content, attrs, .. } => {
                // Add markdown heading prefix based on level
                let heading_prefix = "#".repeat(attrs.level as usize);
                text.push_str(&format!("{} ", heading_prefix));
                // Add a HardBreak to add newline after processing children
                stack.push(&Node::HardBreak { attrs: None });
                for child in content.iter().rev() {
                    stack.push(child);
                }
            }
            Node::Blockquote { content, .. } => {
                // Process blockquote content and prefix each line with "> "
                let blockquote_content = extract_markdown_from_nodes_with_prefix(content, "> ");
                text.push_str(blockquote_content.trim_end());
                text.push_str("\n\n");
            }
            Node::CodeBlock { content, attrs, .. } => {
                // Add language if specified
                if let Some(language) = attrs.as_ref().and_then(|a| a.language.as_ref()) {
                    text.push_str(&format!("```{}\n", language));
                } else {
                    text.push_str("```\n");
                }

                // Process code content directly
                for child in content {
                    if let Node::Text {
                        text: node_text, ..
                    } = child
                    {
                        text.push_str(node_text);
                    }
                }

                // Add closing code block
                text.push_str("\n```\n");
            }
            Node::OrderedList { content, .. } => {
                // Process ordered list items with numbered prefixes
                for (index, item) in content.iter().enumerate() {
                    if let Node::ListItem {
                        content: item_content,
                        ..
                    } = item
                    {
                        let prefix = format!("{}. ", index + 1);
                        let item_text = extract_markdown_from_nodes_with_prefix(
                            item_content,
                            &" ".repeat(prefix.len()),
                        );
                        text.push_str(&prefix);
                        text.push_str(item_text.trim());
                        text.push('\n');
                    }
                }
            }
            Node::BulletList { content, .. } => {
                // Process bullet list items with bullet prefixes
                for item in content.iter() {
                    if let Node::ListItem {
                        content: item_content,
                        ..
                    } = item
                    {
                        let prefix = "- ";
                        let item_text = extract_markdown_from_nodes_with_prefix(
                            item_content,
                            &" ".repeat(prefix.len()),
                        );
                        text.push_str(prefix);
                        text.push_str(item_text.trim());
                        text.push('\n');
                    }
                }
            }
            Node::Table { content, .. } => {
                // Generate markdown table
                let table_markdown = generate_table_markdown(content);
                text.push_str(&table_markdown);
                text.push('\n');
            }
            Node::Panel { content, attrs } => {
                // Generate markdown alert based on panel type
                let alert_type = match attrs.panel_type {
                    PanelType::Info => "NOTE",
                    PanelType::Note => "NOTE",
                    PanelType::Tip => "TIP",
                    PanelType::Warning => "WARNING",
                    PanelType::Error => "CAUTION",
                    PanelType::Success => "IMPORTANT",
                    PanelType::Custom => "NOTE", // Default for custom panels
                };

                text.push_str(&format!("> [!{}]\n", alert_type));

                // Process panel content and prefix each line with "> "
                let panel_content = extract_markdown_from_nodes_with_prefix(content, "> ");
                text.push_str(panel_content.trim_end());
                text.push_str("\n\n");
            }
            Node::Expand { content, attrs, .. } => {
                let title = attrs
                    .as_ref()
                    .and_then(|a| a.title.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("");
                text.push_str(&format!("⬐--- {} ---⬎\n", title));

                let expand_content = extract_markdown_from_nodes(content);
                text.push_str(&expand_content);

                text.push_str(&format!("⬑--- {} ---⬏\n", title));
            }
            Node::NestedExpand { content, .. }
            | Node::MediaGroup { content, .. }
            | Node::MediaSingle { content, .. }
            | Node::BodiedExtension { content, .. } => {
                // Process content without adding newlines
                for child in content.iter().rev() {
                    stack.push(child);
                }
            }
            Node::TableRow { .. } => {
                // TableRow nodes should only be processed within Table contexts
                // Ignore standalone TableRow nodes
            }
            Node::TableCell { .. } => {
                // TableCell nodes should only be processed within TableRow contexts
                // Ignore standalone TableCell nodes
            }
            Node::TableHeader { .. } => {
                // TableHeader nodes should only be processed within TableRow contexts
                // Ignore standalone TableHeader nodes
            }
            Node::DecisionList { content, .. } => {
                // Process decision list items with checkmark prefixes
                for item in content.iter() {
                    if let Node::DecisionItem {
                        content: item_content,
                        ..
                    } = item
                    {
                        let prefix = "✓ ";
                        let item_text = extract_markdown_from_nodes_with_prefix(
                            item_content,
                            &" ".repeat(prefix.len()),
                        );
                        text.push_str(prefix);
                        text.push_str(item_text.trim());
                        text.push('\n');
                    }
                }
            }
            Node::TaskList { content, .. } => {
                // Process task list items with checkbox prefixes
                for item in content.iter() {
                    if let Node::TaskItem {
                        content: item_content,
                        attrs,
                    } = item
                    {
                        let prefix = if matches!(attrs.state, TaskState::Done) {
                            "- [x] "
                        } else {
                            "- [ ] "
                        };
                        let item_text = extract_markdown_from_nodes_with_prefix(
                            item_content,
                            &" ".repeat(prefix.len()),
                        );
                        text.push_str(prefix);
                        text.push_str(item_text.trim());
                        text.push('\n');
                    }
                }
            }
            Node::ListItem { .. } => {
                // ListItem nodes should only be processed within BulletList or OrderedList contexts
                // Ignore standalone ListItem nodes
            }
            Node::TaskItem { .. } => {
                // TaskItem nodes should only be processed within TaskList contexts
                // Ignore standalone TaskItem nodes
            }
            Node::DecisionItem { .. } => {
                // DecisionItem nodes should only be processed within DecisionList contexts
                // Ignore standalone DecisionItem nodes
            }
            Node::Mention { attrs } => {
                if let Some(mention_text) = &attrs.text {
                    text.push_str(mention_text);
                } else {
                    text.push_str(&format!("@{}", attrs.id));
                }
            }
            Node::Emoji { attrs } => {
                if let Some(emoji_text) = &attrs.text {
                    text.push_str(emoji_text);
                } else {
                    text.push_str(&attrs.short_name);
                }
            }
            Node::Date { attrs } => {
                text.push_str(&attrs.timestamp);
            }
            Node::Status { attrs } => {
                text.push_str(&attrs.text);
            }
            Node::Rule { .. } => {
                text.push_str("---\n");
            }
            // Inline cards, block cards, embed cards - show their URLs if available
            Node::InlineCard { attrs } => {
                if let InlineCardAttrs::Url { url, .. } = attrs {
                    text.push_str(&format!("[{}]({})", url, url));
                }
            }
            Node::BlockCard { attrs } => {
                if let BlockCardAttrs::Url { url, .. } = attrs {
                    text.push_str(url)
                }
                stack.push(&Node::HardBreak { attrs: None });
            }
            Node::EmbedCard { attrs } => {
                text.push_str(&attrs.url);
                stack.push(&Node::HardBreak { attrs: None });
            }
            // Media nodes - show alt text if available
            Node::Media { attrs, .. } => match attrs {
                MediaAttrs::File {
                    alt,
                    collection: url,
                    ..
                }
                | MediaAttrs::External { alt, url, .. } => {
                    if let Some(alt_text) = alt {
                        text.push_str(&format!("![{}]({})", alt_text, url));
                    }
                }
            },
            // Extensions - show text if available
            Node::Extension { attrs, .. } | Node::InlineExtension { attrs, .. } => {
                if let Some(extension_text) = &attrs.text {
                    text.push_str(extension_text);
                }
            }
            // Ignore unknown nodes and nodes without text content
            Node::Unknown => {}
        }
    }

    text
}

/// Extract markdown from nodes and add prefix to each line
fn extract_markdown_from_nodes_with_prefix(nodes: &[Node], prefix: &str) -> String {
    let content = extract_markdown_from_nodes(nodes);
    content
        .lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Apply markdown formatting based on text marks
fn apply_text_marks(text: &str, marks: &[Mark]) -> String {
    let mut formatted = text.to_string();

    for mark in marks {
        match mark {
            Mark::Strong => {
                formatted = format!("**{}**", formatted);
            }
            Mark::Em => {
                formatted = format!("*{}*", formatted);
            }
            Mark::Code => {
                formatted = format!("`{}`", formatted);
            }
            Mark::Strike => {
                formatted = format!("~~{}~~", formatted);
            }
            Mark::Underline => {
                formatted = format!("<u>{}</u>", formatted);
            }
            Mark::Link { attrs } => {
                formatted = format!("[{}]({})", formatted, attrs.href);
            }
            Mark::TextColor { attrs } => {
                formatted = format!(
                    "<span style=\"color: {}\">{}</span>",
                    attrs.color, formatted
                );
            }
            _ => {} // Ignore other marks for now
        }
    }

    formatted
}

/// Extract cell content from TableCell or TableHeader node
fn extract_cell_content(cell: &Node) -> Option<String> {
    match cell {
        Node::TableCell { content, .. } | Node::TableHeader { content, .. } => {
            let cell_text = extract_markdown_from_nodes(content).trim().to_string();
            Some(cell_text)
        }
        _ => None,
    }
}

/// Generate markdown table from ADF table nodes
fn generate_table_markdown(table_rows: &[Node]) -> String {
    if table_rows.is_empty() {
        return String::new();
    }

    let mut table_lines = Vec::new();
    let mut rows_iter = table_rows.iter();

    // Handle first row separately to check for headers
    let first_row = rows_iter.next().unwrap();
    if let Node::TableRow { content, .. } = first_row {
        let mut cells = Vec::new();
        let mut has_headers = false;

        // Process first row and check if it has TableHeader cells
        for cell in content {
            if let Some(cell_text) = extract_cell_content(cell) {
                cells.push(cell_text);
                if matches!(cell, Node::TableHeader { .. }) {
                    has_headers = true;
                }
            }
        }

        if !cells.is_empty() {
            let cell_count = cells.len();

            if has_headers {
                // First row has headers - add it and separator
                let row_line = format!("| {} |", cells.join(" | "));
                table_lines.push(row_line);
                let separator = format!(
                    "| {} |",
                    cells.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
                );
                table_lines.push(separator);
            } else {
                // First row has no headers - add empty header and separator first
                let empty_header = format!(
                    "| {} |",
                    (0..cell_count)
                        .map(|_| "   ")
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                let separator = format!(
                    "| {} |",
                    (0..cell_count)
                        .map(|_| "---")
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                table_lines.push(empty_header);
                table_lines.push(separator);
                // Then add the first row
                let row_line = format!("| {} |", cells.join(" | "));
                table_lines.push(row_line);
            }
        }
    }

    // Process remaining rows - treat all cells (TableCell and TableHeader) the same
    for row in rows_iter {
        if let Node::TableRow { content, .. } = row {
            let mut cells = Vec::new();

            for cell in content {
                if let Some(cell_text) = extract_cell_content(cell) {
                    cells.push(cell_text);
                }
            }

            if !cells.is_empty() {
                let row_line = format!("| {} |", cells.join(" | "));
                table_lines.push(row_line);
            }
        }
    }

    table_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_to_markdown_simple() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Paragraph {
                marks: vec![],
                attrs: None,
                content: vec![Node::Text {
                    text: "Hello world".to_string(),
                    marks: vec![],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "Hello world\n");
    }

    #[test]
    fn test_document_to_markdown_heading() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Heading {
                marks: vec![],
                attrs: HeadingAttrs {
                    level: 2,
                    local_id: None,
                },
                content: vec![Node::Text {
                    text: "Section Title".to_string(),
                    marks: vec![],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "## Section Title\n");
    }

    #[test]
    fn test_document_to_markdown_blockquote() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Blockquote {
                attrs: None,
                content: vec![
                    Node::Paragraph {
                        marks: vec![],
                        attrs: None,
                        content: vec![Node::Text {
                            text: "This is the first line of the quote".to_string(),
                            marks: vec![],
                        }],
                    },
                    Node::Paragraph {
                        marks: vec![],
                        attrs: None,
                        content: vec![Node::Text {
                            text: "This is the second line of the quote".to_string(),
                            marks: vec![],
                        }],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(
            markdown,
            "> This is the first line of the quote\n> This is the second line of the quote\n\n"
        );
    }

    #[test]
    fn test_document_to_markdown_code_block() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::CodeBlock {
                marks: vec![],
                attrs: Some(CodeBlockAttrs {
                    language: Some("rust".to_string()),
                    unique_id: None,
                    local_id: None,
                }),
                content: vec![Node::Text {
                    text: "fn main() {}".to_string(),
                    marks: vec![],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn test_document_to_markdown_bullet_list() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::BulletList {
                attrs: None,
                content: vec![
                    Node::ListItem {
                        attrs: None,
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "First item".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                    Node::ListItem {
                        attrs: None,
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "Second item".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "- First item\n- Second item\n");
    }

    #[test]
    fn test_document_to_markdown_ordered_list() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::OrderedList {
                attrs: None,
                content: vec![
                    Node::ListItem {
                        attrs: None,
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "First item".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                    Node::ListItem {
                        attrs: None,
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "Second item".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "1. First item\n2. Second item\n");
    }

    #[test]
    fn test_document_to_markdown_panel() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Panel {
                attrs: PanelAttrs {
                    panel_type: PanelType::Warning,
                    panel_icon: None,
                    panel_icon_id: None,
                    panel_icon_text: None,
                    panel_color: None,
                    local_id: None,
                },
                content: vec![Node::Paragraph {
                    marks: vec![],
                    attrs: None,
                    content: vec![Node::Text {
                        text: "Warning message".to_string(),
                        marks: vec![],
                    }],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "> [!WARNING]\n> Warning message\n\n");
    }

    #[test]
    fn test_document_to_markdown_expand_with_title() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Expand {
                marks: vec![],
                attrs: Some(ExpandAttrs {
                    title: Some("Click to expand".to_string()),
                    local_id: None,
                }),
                content: vec![Node::Paragraph {
                    marks: vec![],
                    attrs: None,
                    content: vec![Node::Text {
                        text: "This content is hidden by default".to_string(),
                        marks: vec![],
                    }],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "⬐--- Click to expand ---⬎\nThis content is hidden by default\n⬑--- Click to expand ---⬏\n");
    }

    #[test]
    fn test_document_to_markdown_expand_without_title() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Expand {
                marks: vec![],
                attrs: None,
                content: vec![Node::Paragraph {
                    marks: vec![],
                    attrs: None,
                    content: vec![Node::Text {
                        text: "This content is hidden by default".to_string(),
                        marks: vec![],
                    }],
                }],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(
            markdown,
            "⬐---  ---⬎\nThis content is hidden by default\n⬑---  ---⬏\n"
        );
    }

    #[test]
    fn test_document_to_markdown_task_list() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::TaskList {
                attrs: TaskListAttrs {
                    local_id: "task-list-1".to_string(),
                },
                content: vec![
                    Node::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "task-1".to_string(),
                            state: TaskState::Todo,
                        },
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "Unchecked task".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                    Node::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "task-2".to_string(),
                            state: TaskState::Done,
                        },
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "Completed task".to_string(),
                                marks: vec![],
                            }],
                        }],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "- [ ] Unchecked task\n- [x] Completed task\n");
    }

    #[test]
    fn test_document_to_markdown_table() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Table {
                marks: vec![],
                attrs: None,
                content: vec![
                    Node::TableRow {
                        attrs: None,
                        content: vec![
                            Node::TableHeader {
                                attrs: None,
                                content: vec![Node::Paragraph {
                                    marks: vec![],
                                    attrs: None,
                                    content: vec![Node::Text {
                                        text: "Header 1".to_string(),
                                        marks: vec![],
                                    }],
                                }],
                            },
                            Node::TableHeader {
                                attrs: None,
                                content: vec![Node::Paragraph {
                                    marks: vec![],
                                    attrs: None,
                                    content: vec![Node::Text {
                                        text: "Header 2".to_string(),
                                        marks: vec![],
                                    }],
                                }],
                            },
                        ],
                    },
                    Node::TableRow {
                        attrs: None,
                        content: vec![
                            Node::TableCell {
                                attrs: None,
                                content: vec![Node::Paragraph {
                                    marks: vec![],
                                    attrs: None,
                                    content: vec![Node::Text {
                                        text: "Cell 1".to_string(),
                                        marks: vec![],
                                    }],
                                }],
                            },
                            Node::TableCell {
                                attrs: None,
                                content: vec![Node::Paragraph {
                                    marks: vec![],
                                    attrs: None,
                                    content: vec![Node::Text {
                                        text: "Cell 2".to_string(),
                                        marks: vec![],
                                    }],
                                }],
                            },
                        ],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(
            markdown,
            "| Header 1 | Header 2 |\n| --- | --- |\n| Cell 1 | Cell 2 |\n"
        );
    }

    #[test]
    fn test_document_to_markdown_mention() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Paragraph {
                marks: vec![],
                attrs: None,
                content: vec![
                    Node::Text {
                        text: "Hello ".to_string(),
                        marks: vec![],
                    },
                    Node::Mention {
                        attrs: MentionAttrs {
                            id: "user123".to_string(),
                            text: Some("@john.doe".to_string()),
                            access_level: None,
                            user_type: None,
                            local_id: None,
                        },
                    },
                    Node::Text {
                        text: " how are you?".to_string(),
                        marks: vec![],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "Hello @john.doe how are you?\n");
    }

    #[test]
    fn test_document_to_markdown_mention_without_text() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![Node::Paragraph {
                marks: vec![],
                attrs: None,
                content: vec![
                    Node::Text {
                        text: "Hello ".to_string(),
                        marks: vec![],
                    },
                    Node::Mention {
                        attrs: MentionAttrs {
                            id: "user123".to_string(),
                            text: None,
                            access_level: None,
                            user_type: None,
                            local_id: None,
                        },
                    },
                    Node::Text {
                        text: " how are you?".to_string(),
                        marks: vec![],
                    },
                ],
            }],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "Hello @user123 how are you?\n");
    }

    #[test]
    fn test_apply_text_marks_strong() {
        let text = "Hello";
        let bold_text = apply_text_marks(text, &[Mark::Strong]);
        assert_eq!(bold_text, "**Hello**");
    }

    #[test]
    fn test_apply_text_marks_em() {
        let text = "Hello";
        let italic_text = apply_text_marks(text, &[Mark::Em]);
        assert_eq!(italic_text, "*Hello*");
    }

    #[test]
    fn test_apply_text_marks_code() {
        let text = "Hello";
        let code_text = apply_text_marks(text, &[Mark::Code]);
        assert_eq!(code_text, "`Hello`");
    }

    #[test]
    fn test_apply_text_marks_strike() {
        let text = "Hello";
        let strikethrough_text = apply_text_marks(text, &[Mark::Strike]);
        assert_eq!(strikethrough_text, "~~Hello~~");
    }

    #[test]
    fn test_apply_text_marks_underline() {
        let text = "Hello";
        let underline_text = apply_text_marks(text, &[Mark::Underline]);
        assert_eq!(underline_text, "<u>Hello</u>");
    }

    #[test]
    fn test_apply_text_marks_link() {
        let text = "Hello";
        let link_text = apply_text_marks(
            text,
            &[Mark::Link {
                attrs: LinkAttrs {
                    href: "https://example.com".to_string(),
                    title: None,
                    id: None,
                    collection: None,
                    occurrence_key: None,
                },
            }],
        );
        assert_eq!(link_text, "[Hello](https://example.com)");
    }

    #[test]
    fn test_apply_text_marks_text_color() {
        let text = "Hello";
        let colored_text = apply_text_marks(
            text,
            &[Mark::TextColor {
                attrs: TextColorAttrs {
                    color: "#ff0000".to_string(),
                },
            }],
        );
        assert_eq!(colored_text, "<span style=\"color: #ff0000\">Hello</span>");
    }

    #[test]
    fn test_complex_document_to_markdown() {
        let document = Document {
            document_type: "doc".to_string(),
            version: 1,
            content: vec![
                Node::Heading {
                    marks: vec![],
                    attrs: HeadingAttrs {
                        level: 1,
                        local_id: None,
                    },
                    content: vec![Node::Text {
                        text: "Main Title".to_string(),
                        marks: vec![],
                    }],
                },
                Node::Paragraph {
                    marks: vec![],
                    attrs: None,
                    content: vec![
                        Node::Text {
                            text: "This is ".to_string(),
                            marks: vec![],
                        },
                        Node::Text {
                            text: "bold text".to_string(),
                            marks: vec![Mark::Strong],
                        },
                        Node::Text {
                            text: " and this is ".to_string(),
                            marks: vec![],
                        },
                        Node::Text {
                            text: "italic text".to_string(),
                            marks: vec![Mark::Em],
                        },
                        Node::Text {
                            text: ".".to_string(),
                            marks: vec![],
                        },
                    ],
                },
                Node::BulletList {
                    attrs: None,
                    content: vec![Node::ListItem {
                        attrs: None,
                        content: vec![Node::Paragraph {
                            marks: vec![],
                            attrs: None,
                            content: vec![Node::Text {
                                text: "List item with code".to_string(),
                                marks: vec![Mark::Code],
                            }],
                        }],
                    }],
                },
            ],
        };

        let markdown = document.to_markdown();
        assert_eq!(markdown, "# Main Title\nThis is **bold text** and this is *italic text*.\n- `List item with code`\n");
    }
}

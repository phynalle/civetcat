#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "cameCase")]
pub struct RawRule {
    pub id: Cell<Option<usize>>,
    pub include: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "match")]
    pub match_expr: Option<String>,
    pub captures: Option<HashMap<usize, RawRule>>,
    pub begin: Option<String>,
    pub begin_captures: Option<HashMap<usize, RawRule>>,
    pub end: Option<String>,
    pub end_captures: Option<HashMap<usize, RawRule>>,
    pub patterns: Option<Vec<RawRule>>,
    pub repository: Option<HashMap<String, RawRule>>,
}

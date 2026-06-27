use compact_str::CompactString;
use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct FontFace {
//     pub family: CompactString,
//     pub src: CompactString,
//     #[serde(default)]
//     pub format: Option<CompactString>,
//     #[serde(default)]
//     pub weight: Option<CompactString>,
//     #[serde(default)]
//     pub style: Option<CompactString>,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeMeta {
    pub name: CompactString,
    #[serde(default)]
    pub author: CompactString,
    #[serde(default)]
    pub version: CompactString,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Theme {
    colors: Colors,
    background: Background,
    text: Text,
    fonts: Vec<Font>,
    layout: Layout,
    borders: Borders,
    shadows: Shadows,
    others: Others    
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Colors {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Background {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Text {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Font {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layout {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Borders {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shadows {}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Others {}
use crate::settings::{LANGUAGE_ENV_PRIORITY, TRADITIONAL_CHINESE_TAGS};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Locale {
    En,
    ZhTw,
}

impl Locale {
    pub fn from_env() -> Self {
        Self::resolve(|name| std::env::var(name).ok())
    }

    pub fn resolve(lookup: impl Fn(&str) -> Option<String>) -> Self {
        LANGUAGE_ENV_PRIORITY
            .iter()
            .find_map(|name| lookup(name).filter(|value| !value.is_empty()))
            .map_or(Self::En, |value| Self::parse(&value))
    }

    pub fn parse(tag: &str) -> Self {
        let tag = tag.to_ascii_lowercase().replace('_', "-");
        let language = tag.split(['.', '@']).next().unwrap_or_default();
        if TRADITIONAL_CHINESE_TAGS
            .iter()
            .any(|prefix| language.starts_with(prefix))
        {
            Self::ZhTw
        } else {
            Self::En
        }
    }
}

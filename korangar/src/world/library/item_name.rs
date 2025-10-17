use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use ragnarok_packets::ItemId;

use super::item_info::ItemInfo;
use super::{Library, Table};
use crate::loaders::GameFileLoader;

static DEFAULT_STRING: &str = "NOTFOUND";
static DEFAULT_VALUE: ItemName = ItemName(Cow::Borrowed(DEFAULT_STRING));

#[derive(Debug, Clone, Copy)]
pub struct ItemNameKey {
    pub item_id: ItemId,
    pub is_identified: bool,
}

#[derive(Debug, Clone)]
pub struct ItemName(Cow<'static, str>);

impl Display for ItemName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ItemName {
    pub(super) const fn not_found_value() -> Self {
        ItemName(Cow::Borrowed(DEFAULT_STRING))
    }

    pub(super) fn from_option(value: Option<String>) -> Self {
        match value {
            Some(s) => ItemName(Cow::Owned(s)),
            None => ItemName::not_found_value(),
        }
    }
}

impl Table for ItemName {
    type Key<'a> = ItemNameKey;
    type Storage = ();

    fn load(_game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        Ok(())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        let item_info = ItemInfo::try_get(library, key.item_id)?;
        Some(if key.is_identified {
            &item_info.identified_name
        } else {
            &item_info.unidentified_name
        })
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        Self::try_get(library, key).unwrap_or(&DEFAULT_VALUE)
    }
}

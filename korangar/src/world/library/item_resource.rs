use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use ragnarok_packets::ItemId;

use super::item_info::ItemInfo;
use super::{Library, Table};
use crate::loaders::GameFileLoader;

static DEFAULT_STRING: &str = "사과"; // Apple
static DEFAULT_VALUE: ItemResource = ItemResource(Cow::Borrowed(DEFAULT_STRING));

#[derive(Debug, Clone, Copy)]
pub struct ItemResourceKey {
    pub item_id: ItemId,
    pub is_identified: bool,
}

#[derive(Debug, Clone)]
pub struct ItemResource(Cow<'static, str>);

impl Display for ItemResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ItemResource {
    pub(super) const fn not_found_value() -> Self {
        ItemResource(Cow::Borrowed(DEFAULT_STRING))
    }

    pub(super) fn from_option(value: Option<String>) -> Self {
        match value {
            Some(s) => ItemResource(Cow::Owned(s)),
            None => ItemResource::not_found_value(),
        }
    }
}

impl Table for ItemResource {
    type Key<'a> = ItemResourceKey;
    type Storage = ();

    fn load(_game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        Ok(())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        let item_info = ItemInfo::try_get(library, key.item_id)?;
        Some(if key.is_identified {
            &item_info.identified_resource
        } else {
            &item_info.unidentified_resource
        })
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        Self::try_get(library, key).unwrap_or(&DEFAULT_VALUE)
    }
}

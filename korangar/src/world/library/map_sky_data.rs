use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::{Lua, Value};

use super::{Library, Table};
use crate::graphics::Color;
use crate::loaders::GameFileLoader;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct MapSkyData {
    old_cloud_effect: Option<usize>,
    bg_color: Option<Color>,
    bg_fog: bool,
    star_effect: bool,
    cloud_effect: Vec<CloudEffect>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct CloudEffect {
    num: usize,
    cull_dist: usize,
    color: Color,
    size: usize,
    size_extra: usize,
    expand_rate: f32,
    alpha_inc_time: usize,
    alpha_inc_time_extra: usize,
    alpha_inc_speed: usize,
    alpha_dec_time: usize,
    alpha_dec_time_extra: usize,
    alpha_dec_speed: f32,
    height: usize,
    height_extra: usize,
}

impl Table for MapSkyData {
    type Key<'a> = &'a str;
    type Storage = HashMap<String, MapSkyData>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let map_sky_data_table = match game_file_loader.get("data\\luafiles514\\lua files\\mapskydata\\mapskydata.lub") {
            Ok(data) => {
                let state = Lua::new();
                state.load(&data).exec()?;

                let globals = state.globals();
                let mut result = HashMap::new();

                if let Ok(table) = globals.get::<mlua::Table>("MapSkyData") {
                    for (map_rsw, map_sky_data_table) in table.pairs::<String, mlua::Table>().flatten() {
                        let resource_name = map_rsw.strip_suffix(".rsw").unwrap_or(&map_rsw).to_string();
                        let map_sky_data = Self::parse_map_sky_data(&map_sky_data_table);
                        result.insert(resource_name, map_sky_data);
                    }
                }

                HashMap::from_iter(result)
            }
            Err(_) => HashMap::new(),
        };

        Ok(map_sky_data_table)
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.map_sky_data_table.get(key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: MapSkyData = MapSkyData {
            old_cloud_effect: None,
            bg_color: None,
            bg_fog: false,
            star_effect: false,
            cloud_effect: vec![],
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

impl MapSkyData {
    fn parse_map_sky_data(table: &mlua::Table) -> MapSkyData {
        let mut cloud_effect = Vec::new();

        if let Ok(Value::Table(cloud_effect_table)) = table.get("Cloud_Effect") {
            let _ = cloud_effect_table.for_each(|_key: usize, value: mlua::Table| {
                let effect = CloudEffect {
                    num: value.get("Num").unwrap_or_default(),
                    cull_dist: value.get("CullDist").unwrap_or_default(),
                    color: value.get("Color").unwrap_or_default(),
                    size: value.get("Size").unwrap_or_default(),
                    size_extra: value.get("Size_Extra").unwrap_or_default(),
                    expand_rate: value.get("Expand_Rate").unwrap_or_default(),
                    alpha_inc_time: value.get("Alpha_Inc_Time").unwrap_or_default(),
                    alpha_inc_time_extra: value.get("Alpha_Inc_Time_Extra").unwrap_or_default(),
                    alpha_inc_speed: value.get("Alpha_Inc_Speed").unwrap_or_default(),
                    alpha_dec_time: value.get("Alpha_Dec_Time").unwrap_or_default(),
                    alpha_dec_time_extra: value.get("Alpha_Dec_Time_Extra").unwrap_or_default(),
                    alpha_dec_speed: value.get("Alpha_Dec_Speed").unwrap_or_default(),
                    height: value.get("Height").unwrap_or_default(),
                    height_extra: value.get("Height_Extra").unwrap_or_default(),
                };
                cloud_effect.push(effect);

                Ok(())
            });
        }

        MapSkyData {
            old_cloud_effect: table.get::<[usize; 1]>("Old_Cloud_Effect").ok().map(|effect| effect[0]),
            bg_color: table.get("BG_Color").ok(),
            bg_fog: table.get("BG_Fog").unwrap_or(false),
            star_effect: table.get("Star_Effect").unwrap_or(false),
            cloud_effect,
        }
    }
}

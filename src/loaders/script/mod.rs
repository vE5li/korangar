use mlua::Lua;

use crate::loaders::GameFileLoader;

pub struct ScriptLoader {
    state: Lua,
}

impl ScriptLoader {

    pub fn new(game_file_loader: &mut GameFileLoader) -> Self {

        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\jobidentity.lub")
            .unwrap();
        state.load(&data).exec().unwrap();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\iteminfo.lub")
            .unwrap();
        state.load(&data).exec().unwrap();

        let job_id_function = r#"
function get_job_name_from_id(id)
  for k,v in pairs(JTtbl) do
    if v==id then

        if string.sub(k, 1, 5) == "JT_G_" then
            return string.sub(k, 6)
        end

        if string.sub(k, 1, 6) == "JT_C1_" then
            return string.sub(k, 7)
        end

        return string.sub(k, 4)
    end
  end
  return "1_f_maria"
  --return nil
end
"#;

        state.load(job_id_function).exec().unwrap();

        Self { state }
    }

    // TODO: move this to a different class that utilizes the script loader
    pub fn get_job_name_from_id(&self, job_id: usize) -> String {

        use mlua::prelude::*;
        use mlua::Function;

        let globals = self.state.globals();

        let print: Function = globals.get("get_job_name_from_id").unwrap();
        print
            .call::<_, LuaString>(job_id)
            .unwrap()
            .to_str()
            .unwrap()
            .replace("CHONCHON", "chocho") // TODO: find a way to do this properly
    }

    // TODO: move this to a different class that utilizes the script loader
    pub fn get_item_name_from_id(&self, item_id: usize) -> String {

        use mlua::prelude::*;

        let globals = self.state.globals();

        globals
            .get::<_, LuaTable>("tbl")
            .unwrap()
            .get::<_, LuaTable>(item_id)
            .unwrap()
            .get::<_, LuaString>("unidentifiedDisplayName")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }
}

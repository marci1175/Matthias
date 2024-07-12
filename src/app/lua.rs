use mlua::Lua;

pub fn execute_code(lua: &Lua, code: String) -> anyhow::Result<()> {
    //Execute code
    lua.load(code).exec()?;

    Ok(())
}

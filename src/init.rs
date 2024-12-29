use mlua::prelude::*;
use smol::stream::StreamExt;

use crate::{process, unix};

async fn pid(_lua: Lua, _: ()) -> LuaResult<u32> {
    Ok(std::process::id())
}

async fn sleep(_lua: Lua, n: u64) -> LuaResult<u64> {
    smol::Timer::after(std::time::Duration::from_secs(n)).await;
    Ok(n)
}

async fn every(_lua: Lua, (n, func, args): (u64, LuaFunction, LuaMultiValue)) -> LuaResult<()> {
    smol::spawn(async move {
        let mut timer = smol::Timer::interval(std::time::Duration::from_secs(n));
        while let Some(_instant) = timer.next().await {
            func.call_async::<()>(args.clone()).await?;
        }
        Ok::<_, LuaError>(())
    })
    .detach();
    Ok(())
}

async fn kill(_lua: Lua, (pid, sig): (i32, i32)) -> LuaResult<i32> {
    unix::kill(pid, sig)
        .await
        .map_err(|err| LuaError::runtime(err.to_string()))
}

pub async fn init(lua: Lua, _: ()) -> LuaResult<LuaTable> {
    let init = lua.create_table()?;
    init.set("exec", lua.create_async_function(process::exec)?)?;
    init.set("kill", lua.create_async_function(kill)?)?;
    init.set("pid", lua.create_async_function(pid)?)?;
    init.set("sleep", lua.create_async_function(sleep)?)?;
    init.set("every", lua.create_async_function(every)?)?;
    init.set("signal", lua.create_table_from(unix::signal_table())?)?;
    Ok(init)
}

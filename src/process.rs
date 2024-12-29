use std::{ffi::OsStr, os::unix::process::ExitStatusExt, sync::Arc};

use async_signal::Signal;
use mlua::prelude::*;
use smol::{
    io::AsyncReadExt,
    lock::RwLock,
    process::{Child, Stdio},
    stream::StreamExt,
};

use crate::{errors::AppResult, unix};

async fn forward_signals(child: Arc<RwLock<Child>>) -> AppResult<()> {
    let pid = child.read().await.id() as i32;
    let mut signals = unix::signal_wait().await?;
    while let Some(signal) = signals.next().await {
        let sig = signal? as i32;
        unix::kill(pid, sig).await?;
    }
    Ok(())
}

async fn spawn<S, I>(program: S, args: I) -> std::io::Result<Child>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
{
    let mut cmd = smol::process::Command::new(&program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.spawn()
}

async fn lua_spawn(_lua: &Lua, cmd: String, args: LuaMultiValue) -> LuaResult<Child> {
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.to_string())
        .collect::<LuaResult<_>>()?;
    Ok(spawn(cmd, args).await?)
}

enum StdStream {
    Stdout,
    Stderr,
}

async fn stream_to_lua_string(
    lua: Lua,
    child: Arc<RwLock<Child>>,
    stream: StdStream,
) -> LuaResult<LuaValue> {
    let mut result = Vec::new();
    let mut child = child.write().await;
    child.stdin.take();
    match stream {
        StdStream::Stdout => {
            child
                .stdout
                .as_mut()
                .ok_or(LuaError::runtime("failed to get stdout"))?
                .read_to_end(&mut result)
                .await?;
        }
        StdStream::Stderr => {
            child
                .stderr
                .as_mut()
                .ok_or(LuaError::runtime("failed to get stderr"))?
                .read_to_end(&mut result)
                .await?;
        }
    }
    let result = match result.is_empty() {
        true => mlua::Value::Nil,
        false => {
            let output = lua.create_string(result.trim_ascii_end())?;
            mlua::Value::String(output)
        }
    };
    Ok(result)
}

pub async fn exec(lua: Lua, (cmd, args): (String, LuaMultiValue)) -> LuaResult<LuaTable> {
    let child = lua_spawn(&lua, cmd, args).await?;
    let child = Arc::new(RwLock::new(child));

    smol::spawn(forward_signals(child.clone())).detach();

    let result = lua.create_table()?;

    // pid
    let clone = child.clone();
    result.set(
        "pid",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move { Ok(child.read().await.id()) }
        })?,
    )?;

    // status
    let clone = child.clone();
    result.set(
        "status",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move {
                let status = child.write().await.status().await?;
                let code = status
                    .signal()
                    .or_else(|| status.code())
                    .ok_or(LuaError::runtime("failed to get status code".to_string()))?;
                Ok(code)
            }
        })?,
    )?;

    // stdout
    let clone = child.clone();
    result.set(
        "stdout",
        lua.create_async_function(move |lua, ()| {
            let child = clone.clone();
            async move { stream_to_lua_string(lua, child, StdStream::Stdout).await }
        })?,
    )?;

    // stderr
    let clone = child.clone();
    result.set(
        "stderr",
        lua.create_async_function(move |lua, ()| {
            let child = clone.clone();
            async move { stream_to_lua_string(lua, child, StdStream::Stderr).await }
        })?,
    )?;

    // kill
    let clone = child.clone();
    result.set(
        "kill",
        lua.create_async_function(move |_, ()| {
            let child = clone.clone();
            async move {
                child.write().await.kill()?;
                Ok(Signal::Kill as i32)
            }
        })?,
    )?;

    Ok(result)
}

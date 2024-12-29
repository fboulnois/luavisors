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

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_setup_spawn() -> std::io::Result<Child> {
        spawn("rustc", ["--version"]).await
    }

    async fn test_setup_exec(lua: &Lua) -> LuaResult<LuaTable> {
        let cmd = "rustc".to_string();
        let args = LuaMultiValue::new();
        exec(lua.clone(), (cmd, args)).await
    }

    #[test]
    fn test_spawn() {
        smol::block_on(async {
            let mut child = test_setup_spawn().await.unwrap();
            let status = child.status().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn test_lua_spawn() {
        smol::block_on(async {
            let lua = Lua::new();
            let cmd = "rustc".to_string();
            let args = LuaMultiValue::from(vec![LuaValue::String(
                lua.create_string("--version").unwrap(),
            )]);
            let mut child = lua_spawn(&lua, cmd, args).await.unwrap();
            let status = child.status().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn test_stream_to_lua_string_stdout() {
        smol::block_on(async {
            let lua = Lua::new();
            let child = test_setup_spawn().await.unwrap();
            let arc = Arc::new(RwLock::new(child));
            let stdout = stream_to_lua_string(lua.clone(), arc.clone(), StdStream::Stdout)
                .await
                .unwrap()
                .to_string()
                .unwrap();
            assert!(stdout.starts_with("rustc"));
        });
    }

    #[test]
    fn test_stream_to_lua_string_stderr() {
        smol::block_on(async {
            let lua = Lua::new();
            let child = test_setup_spawn().await.unwrap();
            let arc = Arc::new(RwLock::new(child));
            let stderr = stream_to_lua_string(lua.clone(), arc.clone(), StdStream::Stderr)
                .await
                .unwrap();
            assert_eq!(stderr, LuaValue::Nil);
        });
    }

    #[test]
    fn test_exec() {
        smol::block_on(async {
            let lua = Lua::new();
            assert!(test_setup_exec(&lua).await.is_ok());
        });
    }

    #[test]
    fn test_exec_pid() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let pid = table.get::<LuaFunction>("pid").unwrap();
            assert!(pid.call_async::<i32>(()).await.is_ok());
        });
    }

    #[test]
    fn test_exec_status() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let status = table.get::<LuaFunction>("status").unwrap();
            assert!(status.call_async::<i32>(()).await.is_ok());
        });
    }

    #[test]
    fn test_exec_stdout() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let stdout = table.get::<LuaFunction>("stdout").unwrap();
            assert!(stdout
                .call_async::<Option<String>>(())
                .await
                .unwrap()
                .is_some());
        });
    }

    #[test]
    fn test_exec_stderr() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let stderr = table.get::<LuaFunction>("stderr").unwrap();
            // stderr is empty and returns nil
            assert!(stderr
                .call_async::<Option<String>>(())
                .await
                .unwrap()
                .is_none());
        });
    }

    #[test]
    fn test_exec_kill() {
        smol::block_on(async {
            let lua = Lua::new();
            let table = test_setup_exec(&lua).await.unwrap();
            let kill = table.get::<LuaFunction>("kill").unwrap();
            assert!(kill.call_async::<i32>(()).await.is_ok());
        });
    }
}

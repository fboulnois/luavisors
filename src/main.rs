#![deny(unsafe_code)]
#![doc = include_str!("../README.md")]

use mlua::{prelude::*, AsChunk};

use crate::{
    errors::{AppResult, NotFoundExt},
    init::init,
};

/// Error handling functions
mod errors;
/// Contains the `init` Lua module
mod init;
/// Process management functions
mod process;
/// Unix-specific functions
mod unix;

/// Print usage information
async fn help() -> AppResult<()> {
    let path = std::env::current_exe()?;
    let exe = path
        .file_name()
        .ok_or_not_found("invalid program name")?
        .to_str()
        .ok_or_not_found("invalid program name")?;
    println!("Usage: {} [script [args...]]", exe);
    Ok(())
}

/// Lua code or path to Lua script
enum Chunk {
    Code(String),
    Path(std::path::PathBuf),
}

/// Convert Lua chunk to bytes
impl<'a> AsChunk<'a> for Chunk {
    fn source(self) -> std::io::Result<std::borrow::Cow<'a, [u8]>> {
        match self {
            Chunk::Code(code) => code.source(),
            Chunk::Path(path) => path.source(),
        }
    }
}

/// Convert Lua chunk to a string
impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Chunk::Code(code) => std::fmt::Display::fmt(&code, f),
            Chunk::Path(path) => std::fmt::Display::fmt(&path.display(), f),
        }
    }
}

/// Parse command line arguments
async fn parse_args(lua: &Lua, args: Vec<String>) -> AppResult<(Chunk, LuaTable)> {
    // find position of lua script in args
    let pos = args.iter().position(|arg| arg.ends_with(".lua"));
    let (chunk, pos) = match pos {
        Some(pos) => (Chunk::Path(std::path::PathBuf::from(&args[pos])), pos),
        None => (Chunk::Code(args[1].clone()), 1),
    };
    // create lua table of arguments
    let table = lua.create_table()?;
    for (i, arg) in args.into_iter().enumerate() {
        let k = i as i32 - pos as i32;
        table.set(k, arg)?;
    }
    Ok((chunk, table))
}

/// Create a new Lua state which allows unsafe code
#[allow(unsafe_code)]
async fn unsafe_lua() -> Lua {
    // SAFETY: allows use of the luajit ffi and c modules
    unsafe { Lua::unsafe_new() }
}

/// Initialize Lua state with `init` module and `arg` table and run the chunk
async fn lua(args: Vec<String>) -> AppResult<()> {
    let lua = unsafe_lua().await;
    // add init table to package preload
    let preload = lua
        .globals()
        .get::<LuaTable>("package")?
        .get::<LuaTable>("preload")?;
    preload.set("init", lua.create_async_function(init)?)?;
    // parse command line arguments
    let (chunk, arg) = parse_args(&lua, args).await?;
    lua.globals().set("arg", arg)?;
    // load and execute the lua script
    lua.load(chunk).exec_async().await?;
    Ok(())
}

/// Execute the program with command line arguments
fn run(args: Vec<String>) -> AppResult<()> {
    smol::block_on(async {
        if args.len() > 1 {
            lua(args).await?;
        } else {
            help().await?;
        }
        Ok(())
    })
}

/// Main program entrypoint
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match run(args) {
        Ok(()) => std::process::exit(0),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_chunk() {
        let chunk = Chunk::Code(String::from("print('hello world')"));
        assert!(chunk.source().is_ok());
    }

    #[test]
    fn test_as_chunk_err() {
        let chunk = Chunk::Path(std::path::PathBuf::new());
        assert!(chunk.source().is_err());
    }

    #[test]
    fn test_help() {
        smol::block_on(async {
            help().await.unwrap();
        });
    }

    #[test]
    fn test_parse_args_path() {
        smol::block_on(async {
            let lua = Lua::new();
            let script = "test.lua";
            let args = vec!["test".to_string(), script.to_string()];
            let (chunk, table) = parse_args(&lua, args).await.unwrap();
            let cmd = table.get::<String>(-1).unwrap();
            assert_eq!(chunk.to_string(), script);
            assert_eq!(cmd, "test");
        });
    }

    #[test]
    fn test_parse_args_code() {
        smol::block_on(async {
            let lua = Lua::new();
            let script = "print('hello world')";
            let args = vec!["test".to_string(), script.to_string()];
            let (chunk, table) = parse_args(&lua, args).await.unwrap();
            let cmd = table.get::<String>(-1).unwrap();
            assert_eq!(chunk.to_string(), script);
            assert_eq!(cmd, "test");
        });
    }

    #[test]
    fn test_unsafe_lua() {
        smol::block_on(async {
            let lua = unsafe_lua().await;
            assert!(lua.load("assert(require('ffi'))").exec().is_ok());
        });
    }

    #[test]
    fn test_lua_core() {
        smol::block_on(async {
            let code = "function add(a, b) return a + b end; add(1, 2)";
            let args = vec!["test".to_string(), code.to_string()];
            assert!(lua(args).await.is_ok());
        });
    }

    #[test]
    fn test_run_help() {
        let args = vec!["test".to_string()];
        assert!(run(args).is_ok());
    }

    #[test]
    fn test_run_lua() {
        let code = "function add(a, b) return a + b end; add(1, 2)";
        let args = vec!["test".to_string(), code.to_string()];
        assert!(run(args).is_ok());
    }
}

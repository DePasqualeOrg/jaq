use jaq_all::data::DataKind;
use jaq_all::jaq_core::native::{self, v, Filter, Fun};
use jaq_all::jaq_core::RunPtr;
#[cfg(not(target_os = "wasi"))]
use crate::{filter, run, with_stdout, write, Error, ErrorColor, Runner, Val};
#[cfg(not(target_os = "wasi"))]
use jaq_all::jaq_core::Vars;
#[cfg(not(target_os = "wasi"))]
use rustyline::error::ReadlineError;
#[cfg(not(target_os = "wasi"))]
use rustyline::DefaultEditor;
#[cfg(not(target_os = "wasi"))]
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn funs() -> impl Iterator<Item = Fun<DataKind>> {
    jaq_all::data::funs().chain([native::run::<DataKind>(repl())])
}

#[cfg(not(target_os = "wasi"))]
/// counter that increases for each nested invocation of `repl`
static REPL_DEPTH: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(target_os = "wasi"))]
/// immediately abort REPL if REPL_DEPTH + 1 == REPL_KILL_DEPTH
static REPL_KILL_DEPTH: AtomicUsize = AtomicUsize::new(0);

pub fn repl() -> Filter<RunPtr<DataKind>> {
    ("repl", v(0), |cv| {
        #[cfg(not(target_os = "wasi"))]
        {
            let depth = REPL_DEPTH.fetch_add(1, Ordering::Relaxed);
            let runner = cv.0.data().runner;
            repl_with(runner, depth, |s| match eval(runner, s, cv.1.clone()) {
                Ok(()) => (),
                Err(e) => eprint!("{}", ErrorColor::new(&e, runner.color_err)),
            })
            .unwrap();
            REPL_DEPTH.fetch_sub(1, Ordering::Relaxed);
        }
        #[cfg(target_os = "wasi")]
        {
            let _ = cv;
            eprintln!("repl is not supported on WASI");
        }

        Box::new(core::iter::empty())
    })
}

#[cfg(not(target_os = "wasi"))]
fn eval(runner: &Runner, code: String, input: Val) -> Result<(), Error> {
    let (ctx, filter) =
        filter::parse_compile(&"<repl>".into(), &code, &[], &[]).map_err(Error::Report)?;
    let ctx = Vars::new(ctx);
    let inputs = core::iter::once(Ok(input));
    let writer = &runner.writer;
    with_stdout(|out| run(runner, &filter, ctx, inputs, |v| write(out, writer, &v)))?;
    Ok(())
}

#[cfg(not(target_os = "wasi"))]
fn repl_with(runner: &Runner, depth: usize, f: impl Fn(String)) -> Result<(), ReadlineError> {
    use rustyline::config::{Behavior, Config};
    let config = Config::builder()
        .behavior(Behavior::PreferTerm)
        .auto_add_history(true)
        .build();
    let mut rl = DefaultEditor::with_config(config)?;
    let history = dirs::cache_dir().map(|dir| dir.join("jaq-history"));
    let color = runner.color_stdout();
    let prompt = if color { "\x1b[1m>\x1b[0m" } else { ">" };
    let _ = history.iter().try_for_each(|h| rl.load_history(h));
    let prompt = format!("{}{} ", str::repeat("  ", depth), prompt);
    let mut first = true;
    loop {
        use core::cmp::Ordering::{Equal, Greater, Less};
        match (depth + 1).cmp(&REPL_KILL_DEPTH.load(Ordering::Relaxed)) {
            Equal => break,
            // reset kill depth if we are below the level where REPL was killed
            Less => REPL_KILL_DEPTH.store(0, Ordering::Relaxed),
            // in this case, kill depth is zero; that is, inactive
            Greater => (),
        }
        match rl.readline(&prompt) {
            Ok(line) if line.chars().all(char::is_whitespace) => (),
            Ok(line) => f(line),
            Err(ReadlineError::Interrupted) => (),
            Err(ReadlineError::Eof) => break,
            Err(err) => Err(err)?,
        }
        first = false;
    }
    // only kill subsequent REPLs at this depth if the first command was "^D" (EOF)
    if first {
        REPL_KILL_DEPTH.store(depth + 1, Ordering::Relaxed);
    }
    let _ = history.iter().try_for_each(|h| rl.append_history(h));
    Ok(())
}

use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;

use super::AppState;
use super::command::{ClosureHandler, CommandHandler, Error};
use super::grid::{ORIGIN, Dir};
use super::instruction::Instruction;

/// Convience macro to define a function that returns a CommandHandler
/// trait object with given behavior.
macro_rules! define_command {
    ($name:ident($app:ident, $arg:pat $(=> $t:ty)?) $body:block) => {
        pub(super) fn $name<R: Read, W: Write>() -> Rc<dyn CommandHandler<R, W>> {
            Rc::new(ClosureHandler::new(
                |$app: &mut AppState<R, W>, $arg $(: $t)?| $body
            )) as Rc<dyn CommandHandler<R, W>>
        }
    }
}

define_command!(quit(app, ()) {
    app.quit = true;
    Ok(())
});

define_command!(list(app, ()) {
    app.ui.list_organisms(&app.organisms, app.focus);
    Ok(())
});

define_command!(max(app, ()) {
    if let Some(old) = app.config.max_organisms {
        app.ui.info1(format!("The current organism limit is {}.", old));
    } else {
        app.ui.info1("There is currently no organism limit.");
    }
    Ok(())
});

define_command!(set_max(app, new) {
    app.config.max_organisms = new;
    if let Some(new) = new {
        app.ui.info1(format!("Organism limit set to {}.", new));
    } else {
        app.ui.info1("Removed organism limit.");
    }
    Ok(())
});

define_command!(speed(app, new) {
    if let Some(new) = new {
        if new == 0 {
            Err(Error::ZeroSpeed)
        } else {
            app.config.cycle_frequency = new;
            app.ui.info1(format!("Set the simulation speed to {}ms/cycle.", new));
            Ok(())
        }
    } else {
        app.ui.info1(format!(
            "The current simulation speed is {}ms/cycle.",
            app.config.cycle_frequency));
        Ok(())
    }
});

define_command!(seed(app, ()) {
    app.ui.info1(format!("The RNG seed is {}.", app.config.rng_seed));
    Ok(())
});

define_command!(source(app, path => PathBuf) {
    app.run_commands_in_file(&path);
    Ok(())
});

define_command!(write_error_chance(app, new_chance) {
    if let Some(chance) = new_chance {
        app.grid.write_error_chance = chance;
        if chance == 0 {
            app.ui.info1("Set the write error chance to 0.");
        } else {
            app.ui.info1(format!("Set the write error chance to 1/{}.", chance))
        }
    } else {
        let chance = app.grid.write_error_chance;
        if app.grid.write_error_chance == 0 {
            app.ui.info1("The current write error chance is 0.");
        } else {
            app.ui.info1(format!("The current write error chance is 1/{}.", chance))
        };
    }
    Ok(())
});

define_command!(cycle(app, times) {
    if let Some(n) = times {
        for _ in 0u32..n {
            app.cycle();
        }
        app.ui.info1(format!("Ran {} cycles.", n));
    } else {
        app.cycle();
        app.ui.info1("Ran a cycle.");
    }
    Ok(())
});

define_command!(pause(app, ()) {
    app.toggle_pause();
    Ok(())
});

define_command!(move_(app, (dir, times) => (Dir, Option<u16>)) {
    let times = times.unwrap_or(1);
    for _ in 0..times {
        app.ui.move_selection(dir);
    }
    Ok(())
});

define_command!(write(app, ins => Instruction) {
    if let Some(selection) = app.ui.selection() {
        app.grid.set(app.absolute(selection), ins as u8)
    }
    Ok(())
});

define_command!(insert_line(app, instructions => Vec<Instruction>) {
    let relative = app.ui.selection().unwrap_or(ORIGIN);
    let mut pos = app.absolute(relative);
    for ins in instructions {
        app.grid.set(pos, ins as u8);
        pos = pos.right(app.grid.width());
    }
    app.ui.select(Some(relative.down(app.grid.height())));
    Ok(())
});

define_command!(byte(app, byte) {
    if let Some(selection) = app.ui.selection() {
        app.grid.set(app.absolute(selection), byte);
    }
    Ok(())
});

define_command!(spawn(app, ()) {
    app.spawn_organism();
    Ok(())
});

define_command!(dedup(app, ()) {
    app.dedup_organisms();
    Ok(())
});

define_command!(auto_dedup(app, ()) {
    if let Some(rate) = app.config.dedup_rate {
        app.ui.info1(format!("Deduplication automatically runs every {} cycles.", rate));
    } else {
        app.ui.info1("Automatic deduplication is disabled.");
    }
    Ok(())
});

define_command!(set_auto_dedup(app, new) {
    app.config.dedup_rate = new;
    if let Some(new) = new {
        app.ui.info1(format!("Set deduplication to run every {} cycles.", new));
    } else {
        app.ui.info1("Disabled automatic deduplication.");
    }
    Ok(())
});

define_command!(focus(app, idx) {
    if let Some(o) = app.organisms.get(idx) {
        app.focus = Some(o.id);
        app.ui.info1(format!("Set focus to organism {}.", idx));
    }
    Ok(())
});

define_command!(unfocus(app, ()) {
    app.focus = None;
    Ok(())
});

define_command!(view(app, ()) {
    if let Some(o) = app.get_focused() {
        app.ui.view_offset = o.organism.ip;
    }
    Ok(())
});

define_command!(move_ip(app, (dir, times) => (Dir, Option<u16>)) {
    if let Some(id) = app.focus {
        let grid_width = app.grid.width();
        let grid_height = app.grid.height();
        let o = app.organisms.iter_mut().find(|o| o.id == id).unwrap();
        for _ in 0..times.unwrap_or(1) {
            o.organism.ip = o.organism.ip.move_in(dir, grid_width, grid_height);
        }
    }
    Ok(())
});

define_command!(run(app, instructions => Vec<Instruction>) {
    if let Some(id) = app.focus {
        let mut tried_to_die = false;
        let o = app.organisms.iter_mut().find(|o| o.id == id).unwrap();
        let mut new_organisms = Vec::new();
        for ins in instructions {
            use super::organism::Response;
            match o.organism.run(&mut app.grid, ins) {
                Response::Delay(_) => {}
                Response::Fork(new) => new_organisms.push(new),
                Response::Die => tried_to_die = true,
            }
        }
        for o in new_organisms {
            app.add_organism(o);
        }
        app.ui.info1(if tried_to_die { "Use the :kill command instead. "} else { "Executed." });
    }
    Ok(())
});

define_command!(kill(app, ()) {
    if let Some(id) = app.focus.take() {
        app.organisms.retain(|o| o.id != id);
    }
    Ok(())
});
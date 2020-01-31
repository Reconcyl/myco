use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use crate::grid::{ORIGIN, Dir};
use super::AppState;
use super::command::{ClosureHandler, CommandHandler, Error};
use super::instruction::Instruction;

/// Convience macro to define a function that returns a CommandHandler
/// trait object with given behavior.
macro_rules! define_command {
    ($name:ident($app:ident, $arg:pat $(=> $t:ty)?) $body:block) => {
        pub(super) fn $name<W: Write>() -> Rc<dyn CommandHandler<W>> {
            Rc::new(ClosureHandler::new(
                |$app: &mut AppState<W>, $arg $(: $t)?| $body
            )) as Rc<dyn CommandHandler<W>>
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
    if let Some(old) = app.organisms.max {
        app.ui.info1(format!("The current organism limit is {}.", old));
    } else {
        app.ui.info1("There is currently no organism limit.");
    }
    Ok(())
});

define_command!(set_max(app, new) {
    app.organisms.max = new;
    if let Some(new) = new {
        app.ui.info1(format!("Organism limit set to {}.", new));
    } else {
        app.ui.info1("Removed organism limit.");
    }
    Ok(())
});

define_command!(lifespan(app, ()) {
    if let Some(age) = app.organisms.max_age {
        app.ui.info1(format!("Organisms currently live for {} cycles.", age));
    } else {
        app.ui.info1("There is currently no maximum lifetime.");
    }
    Ok(())
});

define_command!(set_lifespan(app, new_max) {
    app.organisms.max_age = new_max;
    if let Some(max) = new_max {
        app.ui.info1(format!("Organisms can now live for only {} cycles.", max));
    } else {
        app.ui.info1("There is now no limit on organism lifetime.");
    }
    Ok(())
});

define_command!(max_children(app, ()) {
    if let Some(max) = app.organisms.max_children {
        app.ui.info1(format!("Organisms can currently have a maximum of {} children.", max));
    } else {
        app.ui.info1("There is currently no limit on the number of children an organism can have.");
    }
    Ok(())
});

define_command!(set_max_children(app, new_max) {
    app.organisms.max_children = new_max;
    if let Some(max) = new_max {
        app.ui.info1(format!("Organisms can now have a maximum of {} children.", max));
    } else {
        app.ui.info1("There is now no limit on the number of children an organism can have.");
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

define_command!(export(app, path) {
    let result = app.write_image_data(path);
    if result.is_ok() {
        app.ui.info1("Exported.");
    }
    result
});

define_command!(export_gif(app, (path, settings) => (PathBuf, Option<(u16, Option<u16>)>)) {
    let (num_frames, step) = settings.unwrap_or((100, None));
    let step = step.unwrap_or(4);
    if num_frames == 0 {
        Err(Error::ZeroGifFrames)
    } else if step == 0 {
        Err(Error::ZeroStep)
    } else {
        app.ui.info1("Exporting...");
        let result = app.write_gif_data(path, num_frames as usize, step as usize);
        if result.is_ok() {
            app.ui.info1("Exported.");
        }
        result
    }
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
        if chance == 0 {
            app.ui.info1("The current write error chance is 0.");
        } else {
            app.ui.info1(format!("The current write error chance is 1/{}.", chance))
        };
    }
    Ok(())
});

define_command!(wall_pierce_chance(app, new_chance) {
    if let Some(chance) = new_chance {
        app.grid.wall_pierce_chance = chance;
        if chance == 0 {
            app.ui.info1("Set the chance of piercing a wall to 0.");
        } else {
            app.ui.info1(format!("Set the chance of piercing a wall to 1/{}.", chance))
        }
    } else {
        let chance = app.grid.wall_pierce_chance;
        if chance == 0 {
            app.ui.info1("The current chance of piercing a wall is 0.");
        } else {
            app.ui.info1(format!("The current chance of piercing a wall is 1/{}.", chance))
        };
    }
    Ok(())
});

define_command!(cosmic_ray_rate(app, new) {
    if let Some(rate) = new {
        app.config.cosmic_ray_rate = rate;
        app.ui.info1(format!("Set cosmic rays to occur {} times per cycle.", rate));
    } else {
        app.ui.info1(format!("Cosmic rays occur {} times per cycle.", app.config.cosmic_ray_rate));
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
    app.ui.move_selection_n(dir, times.unwrap_or(1) as usize);
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
    app.organisms.dedup();
    Ok(())
});

define_command!(auto_dedup(app, new) {
    if let Some(rate) = new {
        app.config.dedup_rate = rate;
        if rate == 0 {
            app.ui.info1("Disabled automatic deduplication.");
        } else {
            app.ui.info1(format!("Enabled automatic deduplication every {} cycles.", rate));
        }
    } else {
        let rate = app.config.dedup_rate;
        if rate == 0 {
            app.ui.info1("Automatic deduplication is disabled.");
        } else {
            app.ui.info1(format!("Automatic deduplication runs every {} cycles.", rate));
        }
    }
    Ok(())
});

define_command!(focus(app, idx) {
    if let Some(idx) = idx {
        if let Some(id) = app.ui.get_listed_id(idx) {
            if app.organisms.alive(id) {
                app.focus = Some(id);
                app.ui.info1(format!("Set focus to organism {}.", idx));
            } else {
                app.ui.info1("That organism is not longer alive.");
            }
        } else {
            app.ui.info1("Out of bounds.");
        }
    } else {
        app.ui.info1("Unset focus.");
        app.focus = None;
    }
    Ok(())
});

define_command!(view(app, ()) {
    if let Some(context) = app.organisms.get_opt(app.focus) {
        app.ui.view_offset = context.organism.ip;
    }
    Ok(())
});

define_command!(move_ip(app, (dir, times) => (Dir, Option<u16>)) {
    if let Some(context) = app.organisms.get_opt_mut(app.focus) {
        let grid_width = app.grid.width();
        let grid_height = app.grid.height();
        let n = times.unwrap_or(1) as usize;
        context.organism.ip = context.organism.ip.move_in_n(dir, n, grid_width, grid_height);
    }
    Ok(())
});

define_command!(run(app, instructions => Vec<Instruction>) {
    if let Some(context) = app.organisms.get_opt_mut(app.focus) {
        let mut tried_to_die = false;
        let mut new_organisms = Vec::new();
        for ins in instructions {
            use super::organism::Response;
            match context.organism.run(&mut app.grid, ins) {
                Response::Delay(_) => {}
                Response::Fork(new) => new_organisms.push(new),
                Response::Die => tried_to_die = true,
            }
        }
        for o in new_organisms {
            app.organisms.insert(o);
        }
        app.ui.info1(if tried_to_die { "Use the :kill command instead. "} else { "Executed." });
    }
    Ok(())
});

define_command!(kill(app, ()) {
    if let Some(id) = app.focus.take() {
        app.organisms.remove(id);
    }
    Ok(())
});
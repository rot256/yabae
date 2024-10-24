use std::cmp::max;

use anyhow::Ok;

use yabai::{self, SpaceInfo};

#[derive(Debug, Default)]
struct State {}

fn is_active(space: &SpaceInfo) -> bool {
    space.has_focus || space.is_visible || !space.windows.is_empty()
}

impl State {
    fn get_space_index(&self, index: u32) -> anyhow::Result<SpaceInfo> {
        // check if label is already in use
        let spaces = yabai::query_spaces()?;
        for space in spaces.iter() {
            if space.index != index {
                continue;
            }

            // check if space is already active
            if is_active(space) {
                println!("space is already active: {:#?}", space);
                return Ok(space.clone());
            }

            // if not active, move to focus display
            let displays = yabai::query_displays()?;
            let has_focus = displays.iter().find(|display| display.has_focus).unwrap();
            if has_focus.index == space.display {
                return Ok(space.clone());
            }

            // move to display
            yabai::send(&format!("space --display {}", has_focus.index))?;

            // try again
            return self.get_space_index(index);
        }

        // create fresh space on active display
        yabai::send(&format!("space --create"))?;

        // try again
        self.get_space_index(index)
    }

    fn clean_spaces_index(&self) -> anyhow::Result<()> {
        // find the top-most active space
        let spaces = yabai::query_spaces()?;
        let mut active_index = 1;
        for space in spaces.iter() {
            if is_active(space) {
                active_index = max(active_index, space.index);
            }
        }

        // destroy all spaces with higher index
        let mut to_destroy = vec![];
        for space in spaces.iter() {
            if space.index <= active_index {
                continue;
            }
            to_destroy.push(space.clone());
        }

        // destroy largest to smallest
        to_destroy.sort_by_key(|space| space.index);
        to_destroy.reverse();
        println!("to_destroy: {:#?}", to_destroy);
        for space in to_destroy.iter() {
            yabai::send(&format!("space --destroy {}", space.index))?;
        }

        Ok(())
    }

    fn goto_space(&self, index: u32) -> anyhow::Result<()> {
        let space = self.get_space_index(index)?;
        yabai::focus_space(space.index)?;
        self.clean_spaces_index()?;
        Ok(())
    }

    fn send_to_space(&self, index: u32) -> anyhow::Result<()> {
        let space = self.get_space_index(index)?;
        yabai::send(&format!("window --space {}", space.index))?;
        self.clean_spaces_index()?;
        Ok(())
    }
}

fn main() {
    // parse arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        std::process::exit(1);
    }

    let state = State::default();
    let command = &args[1];
    let index = args[2].parse::<u32>().unwrap();
    state.clean_spaces_index().unwrap();

    match command.as_str() {
        "goto" => {
            state.goto_space(index).unwrap();
            state.clean_spaces_index().unwrap();
        }
        "send" => {
            state.send_to_space(index).unwrap();
            state.clean_spaces_index().unwrap();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

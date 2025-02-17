use std::cmp::{max, min};

use anyhow::Ok;

use yabai::{self, SpaceInfo};

const MAX_SPACES: u32 = 10;

const INDICATOR_SOCKET: &str = "/tmp/yabai-indicator.socket";

#[derive(Debug, Default)]
struct State {}

fn is_active(space: &SpaceInfo) -> bool {
    space.has_focus || space.is_visible || !space.windows.is_empty()
}

impl State {
    fn get_space_index(&self, index: u32) -> anyhow::Result<SpaceInfo> {
        println!("get_space_index: {}", index);
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
            /*
            let displays = yabai::query_spaces()?;
            let has_focus = displays.iter().find(|display| display.has_focus).unwrap();
            if has_focus.index == space.display {
                return Ok(space.clone());
            }

            // move to display
            yabai::send(&format!("space --display {}", has_focus.index))?;
            */

            // try again
            return Ok(space.clone());
            // return self.get_space_index(index);
        }

        // create fresh space on active display
        yabai::send(&format!("space --create"))?;

        // try again
        self.get_space_index(index)
    }

    fn clean_spaces_index(&self) -> anyhow::Result<()> {
        println!("clean_spaces_index");

        // find the top-most active space
        let spaces = yabai::query_spaces()?;
        println!("spaces: {:#?}", spaces);
        let mut active_index = 1;
        for space in spaces.iter() {
            if is_active(space) {
                active_index = max(active_index, space.index);
            }
        }

        // cap active_index
        let active_index = min(active_index, MAX_SPACES);
        println!("highest active_index: {}", active_index);

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
        println!("goto_space: {}", index);
        let space = self.get_space_index(index)?;
        yabai::focus_space(space.index)?;
        self.clean_spaces_index()?;
        Ok(())
    }

    fn send_to_space(&self, index: u32) -> anyhow::Result<()> {
        println!("send_to_space: {}", index);
        let space = self.get_space_index(index)?;
        yabai::send(&format!("window --space {}", space.index))?;
        self.clean_spaces_index()?;
        Ok(())
    }

    fn refresh(&self) -> anyhow::Result<()> {
        println!("refresh");
        use std::io::Write;
        match std::os::unix::net::UnixStream::connect(INDICATOR_SOCKET) {
            Result::Ok(mut socket) => {
                socket.write_all(b"refresh")?;
            }
            Result::Err(err) => {
                println!("failed to connect to indicator socket: {}", err);
                return Ok(());
            }
        }
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

    match command.as_str() {
        "clean" => {}
        "goto" => {
            let index = args[2].parse::<u32>().unwrap();
            state.goto_space(index).unwrap();
        }
        "send" => {
            let index = args[2].parse::<u32>().unwrap();
            state.send_to_space(index).unwrap();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
    state.clean_spaces_index().unwrap();
    state.refresh().unwrap();
}

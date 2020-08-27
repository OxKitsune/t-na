use chrono::Utc;
use crate::util::colour;

struct LogLevel {
    text: String,
    text_colour: String
}

fn log (level: LogLevel, msg: String) {
    println!("{}[{}] [{}]: {}", level.text_colour,  Utc::now().format("%r %v"), level.text, msg);
}

// Log the message with the info level
pub fn info (msg: String){
    log(LogLevel{text:String::from("INFO"), text_colour:String::from(colour::WHITE)}, msg);
}

// Log the message with the info level
pub fn error (msg: String){
    log(LogLevel{text:String::from("ERROR"), text_colour:String::from(colour::RED)}, msg);
}

// Log the message with the info level
pub fn warn (msg: String){
    log(LogLevel{text:String::from("WARN"), text_colour:String::from(colour::YELLOW)}, msg);
}
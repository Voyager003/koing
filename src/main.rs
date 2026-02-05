use koing::convert;
use std::io::{self, BufRead, Write};

fn main() {
    println!("Koing - macOS 한영 자동변환 프로그램");
    println!("영문을 입력하면 한글로 변환합니다. (종료: Ctrl+D 또는 'quit')");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = input.trim();
                if input == "quit" || input == "exit" {
                    break;
                }
                let output = convert(input);
                println!("  {}", output);
            }
            Err(_) => break,
        }
    }

    println!("\n안녕!");
}

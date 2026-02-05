use koing::convert;

fn main() {
    println!("Koing - macOS 한영 자동변환 프로그램");
    println!();

    // 데모
    let examples = [
        "rkskek",
        "dkssudgktpdy",
        "gksrmf",
        "dhksfy",
        "Tks",
        "123rksk",
        "rk!sk",
    ];

    for input in examples {
        let output = convert(input);
        println!("{} -> {}", input, output);
    }
}

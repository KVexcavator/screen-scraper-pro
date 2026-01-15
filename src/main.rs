/// так я ловлю дисплэи, елсли я включу два то выведет два
use scrap::Display;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let displays = Display::all()?;

    let targets: Vec<String> = displays
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!("Display {}:::{}:::{}", i, d.height(), d.width())
        })
        .collect();

    println!("Доступные цели:");
    for t in &targets {
        println!("  {}", t);
    }

    Ok(())
}

// Доступные цели:
//   Display 0:::1080:::1920
//   Display 1:::1080:::1920

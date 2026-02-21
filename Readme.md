### My OBS
===
#### Cтруктура проекта:
```
.
├── Cargo.toml          # workspace
├── docs/
├── Readme.md
├── ui/                  # Slint crate
├── linux-backend/       # ловим окна на linux
├── windows-backend/     # ловим окна на windows
└── app/                 # общий launcher (опционально)
```
#### linux-backend/src/main.rs или windows-backend/src/main.rs
```rust
use screen_ui::run_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Linux backend");

    // тут portal + pipewire

    run_app()?;
    Ok(())
}
```
#### Slint один и тот же
```bash
cargo run -p linux-backend
cargo run -p windows-backend
```
#### app crate для авто-определение платформы
- пока удобнее запускать по пакетам
===
#### этапы Linux:
- Подключиться к PipeWire node (я почти тут)
- Получить buffer в callback
- Прочитать spa::buffer::Data
- Конвертировать формат (обычно DMA-BUF/YUV)
- Скопировать в CPU (RGBA)
- Передать в Slint Image
===
#### пакеты Linux:
```
sudo apt update
sudo apt install -y \
  libpipewire-0.3-dev \
  libspa-0.2-dev \
  pkg-config

pkg-config --libs libpipewire-0.3

sudo apt update
sudo apt install -y clang libclang-dev
# Проверка, что libclang реально есть
ls /usr/lib/llvm-*/lib/libclang.so*
# Cохранить путь
echo 'export LIBCLANG_PATH=/usr/lib/llvm-18/lib' >> ~/.bashrc
source ~/.bashrc
```
===
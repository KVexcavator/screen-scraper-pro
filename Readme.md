### Stream pack
- десктопное приложение для стриминга

#### Cтруктура проекта:
```
.
├── Cargo.toml          # workspace
├── docs/
├── Readme.md
├── ui/                 # Slint crate
├── linux-backend/      # ловим окна на linux
├── windows-backend/    # ловим окна на windows
└── app/                # общий launcher
```
#### app crate для авто-определение платформы
- Cargo умеет определять ОС и подтягивать нужные зависимости через cfg
```bash
cargo run
```
#### команда добавления пакетов по отделности
```bash
cargo add tokio -p windows-backend --features all
```

#### этапы Linux:
- Подключиться к PipeWire node (я почти тут)
- Получить buffer в callback
- Прочитать spa::buffer::Data
- Конвертировать формат (обычно DMA-BUF/YUV)
- Скопировать в CPU (RGBA)
- Передать в Slint Image

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

### полезные команды:
- отменить все изменения после коммита
```
git reset --hard HEAD~1
```
- автофикс при варнингах
```
cargo clippy --fix --lib -p windows-backend
```


===
sudo apt update
sudo apt install -y \
  libpipewire-0.3-dev \
  libspa-0.2-dev \
  pkg-config

pkg-config --libs libpipewire-0.3

sudo apt update
sudo apt install -y clang libclang-dev
Проверка, что libclang реально есть
ls /usr/lib/llvm-*/lib/libclang.so*
Cохранить путь
echo 'export LIBCLANG_PATH=/usr/lib/llvm-18/lib' >> ~/.bashrc
source ~/.bashrc
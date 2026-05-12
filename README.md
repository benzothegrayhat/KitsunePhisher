Install depencies
```
sudo apt install -y rustc git cargo
```
To run
```
git clone https://github.com/benzothegrayhat/KitsunePhisher
cd KitsunePhisher
cargo build --release
sudo cp target/release/kitsune_phish /usr/local/bin/kitsune
sudo cp -r .sites /usr/local/bin
```

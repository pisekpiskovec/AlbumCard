run:
    cargo run

build:
    cargo build

test:
    cargo test

release:
    cargo build --release

[linux]
install:
    cargo build --release
    cp ./target/release/album-card $HOME/.local/bin/album-card
    # mkdir -p $HOME/.local/share/icons/hicolor/256x256/apps
    # cp assets/icon.png $HOME/.local/share/icons/hicolor/256x256/apps/breadboard.png
    # sed "s|Exec=breadboard|Exec=$HOME/.local/bin/breadboard|" breadboard.desktop > /tmp/breadboard.desktop
    # desktop-file-install --dir=$HOME/.local/share/applications /tmp/breadboard.desktop
    cargo clean

# [linux]
# gen-rpm:
#     cargo install cargo-generate-rpm
#     cargo build --release
#     strip -s target/release/Breadboard
#     cargo generate-rpm
#     mv target/generate-rpm/*.rpm ./
#     cargo clean

# [linux]
# gen-deb:
#     cargo install cargo-deb
#     cargo build --release
#     strip -s target/release/Breadboard
#     cargo deb
#     mv target/debian/*.deb ./
#     cargo clean

[linux]
build-flatpak:
    flatpak-builder --force-clean --disable-rofiles-fuse --user --install-deps-from=flathub --repo=repo --install builddir io.github.pisekpiskovec.AlbumCard.json
    xdg-desktop-menu forceupdate
    flatpak run io.github.pisekpiskovec.AlbumCard

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
    cp ./target/release/albumcard $HOME/.local/bin/albumcard
    mkdir -p $HOME/.local/share/icons/hicolor/scalable/apps
    cp data/io.github.pisekpiskovec.AlbumCard.svg $HOME/.local/share/icons/hicolor/scalable/apps/io.github.pisekpiskovec.AlbumCard.svg
    sed "s|Exec=albumcard|Exec=$HOME/.local/bin/albumcard|" data/io.github.pisekpiskovec.AlbumCard.desktop > /tmp/io.github.pisekpiskovec.AlbumCard.desktop
    desktop-file-install --dir=$HOME/.local/share/applications /tmp/io.github.pisekpiskovec.AlbumCard.desktop
    cargo clean

[linux]
gen-rpm:
    cargo install cargo-generate-rpm
    cargo build --release
    strip -s target/release/albumcard
    cargo generate-rpm
    mv target/generate-rpm/*.rpm ./
    cargo clean

[linux]
gen-deb:
    cargo install cargo-deb
    cargo build --release
    strip -s target/release/albumcard
    cargo deb
    mv target/debian/*.deb ./
    cargo clean

[linux]
build-flatpak:
    flatpak-builder --force-clean --disable-rofiles-fuse --user --install-deps-from=flathub --repo=repo --install builddir io.github.pisekpiskovec.AlbumCard.json
    xdg-desktop-menu forceupdate

[linux]
run-flatpak:
    flatpak-builder --force-clean --disable-rofiles-fuse --user --install-deps-from=flathub --repo=repo --install builddir io.github.pisekpiskovec.AlbumCard.json
    xdg-desktop-menu forceupdate
    flatpak run io.github.pisekpiskovec.AlbumCard

prepare-for-release:
    bash -c scripts/update-cargo-sources.sh

[linux]
flowerpot:
    find src -name '*.rs' -print0 | xargs -0 xgettext --language=Rust --from-code=UTF-8 --output=po/albumcard.pot

# Maintainer: Folio contributors <maintainers@folio.invalid>
pkgname=folio
pkgver=1.0.0
pkgrel=1
pkgdesc="Notas y libros diarios con lienzo y plugins"
arch=('x86_64')
url="https://github.com/Leonisaurov/folio"
license=('GPL-3.0-or-later')
depends=('gtk4' 'libadwaita' 'sqlite' 'zstd' 'gst-plugins-base' 'gst-plugins-good')
makedepends=('rust' 'cargo' 'pkgconf')
optdepends=('webkitgtk-6.0: vista web viva' 'gst-plugins-bad: códecs de medios')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    # makepkg aplica LDFLAGS de Arch; declarar las bibliotecas nativas evita
    # que --as-needed descarte los símbolos de los crates -sys.
    export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=/usr/lib/libsqlite3.so -C link-arg=/usr/lib/libzstd.so"
    cargo build --release --locked --offline -p folio-app
    cargo build --release --locked --offline -p folio-tools
}

check() {
    cd "$pkgname-$pkgver"
    cargo test -p folio-core -p folio-store -p folio-config -p folio-daily         -p folio-layout -p folio-render -p folio-plugins --locked --offline
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 target/release/folio "$pkgdir/usr/bin/folio"
    install -Dm755 target/release/folio-tools "$pkgdir/usr/bin/folio-tools"
    install -Dm644 flatpak/io.github.leonisauro.Folio.desktop \
        "$pkgdir/usr/share/applications/io.github.leonisauro.Folio.desktop"
    install -Dm644 flatpak/io.github.leonisauro.Folio.metainfo.xml \
        "$pkgdir/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml"
    install -Dm644 flatpak/io.github.leonisauro.Folio.svg \
        "$pkgdir/usr/share/icons/hicolor/scalable/apps/io.github.leonisauro.Folio.svg"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 docs/16-manual-usuario.md \
        "$pkgdir/usr/share/doc/$pkgname/manual.md"
    install -Dm644 docs/17-recuperacion.md \
        "$pkgdir/usr/share/doc/$pkgname/recuperacion.md"
    install -Dm644 docs/18-accesibilidad.md \
        "$pkgdir/usr/share/doc/$pkgname/accesibilidad.md"
    install -Dm644 docs/22-release-notes-1.0.0.md \
        "$pkgdir/usr/share/doc/$pkgname/release-notes.md"
    install -d "$pkgdir/usr/share/folio/plugins"
    cp -r plugins/* "$pkgdir/usr/share/folio/plugins/"
}

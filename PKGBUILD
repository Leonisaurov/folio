# Maintainer: Folio contributors <maintainers@folio.invalid>
pkgname=folio
pkgver=0.1.0
pkgrel=1
pkgdesc="Notas y libros diarios con lienzo y plugins"
arch=('x86_64')
url="https://github.com/leonisauro/folio"
license=('GPL-3.0-or-later')
depends=('gtk4' 'libadwaita' 'sqlite' 'gst-plugins-base' 'gst-plugins-good')
makedepends=('rust' 'cargo' 'pkgconf')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release --locked --offline
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --workspace --locked --offline
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
}

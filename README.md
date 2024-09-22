# pdf_to_image_rs
pdfから画像ファイルを取り出すコマンドのrust版。

リリースビルド用コマンド。
```
cargo build --release
```
実行例(カレントディレクトリ内の全pdfファイルから画像を取り出す。)
```
pdf_to_image_rs --pdfdir `pwd`
```

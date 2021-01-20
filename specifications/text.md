# Text

## Servo Code Reading

- `UnscannedText`にはnodeから取り出したtextが含まれている
- `UnscannedText`に含まれているtextを`scan_for_runs`関数で分割し、それぞれのcharの情報を`RunInfo`に格納している
  - 格納する情報は、`font`, `script`(charの言語の種類), `bidi`など
  - `font`は一文字ごとにfontの種類が異なる可能性がある。例えば、`あBう`に`sans-serif`を指定している場合、`sans-serif`には日本語の情報がないため、`Hiragino Sans GB`を個別に適用する必要がある
  - `script`で言語別に`run_info`を分別していて、一つなぎの`katakana`なら同じ`run_info`の`text`に格納される。`script`が異なる場合、`run_info`を`run_info_list`に挿入し、`run_info`をflushする
  - これらの作業はとても重い作業であるため、読み込んだfontは必ずcacheする
- `transform_text`で文字列の空白を圧縮し、`run_info.text`に文字情報を格納する

### Logic

- `flush_clump_to_list()`で1文字づつchをみていき、その文字のfont周りのスタイルを取得していくが、`ch.is_control()`が`false`のときは、この処理を行わない
- スタイルは、bold(font-weight), italicなどを取得し、`run_info`に格納する
- chごとにfont-familyを確認していく。この時に`ucd::UnicodeBlock`を使って適用するfamilyを決定する
- textはscriptごとにまとめられていて、loopの中でscriptが異なっていれば、新しい`run_info`を作成する
- scriptは`unicode_script::Script::from`を使ってscriptを取得する(いったんfontごとに分類する感じでいいかも)
- スタイルの適用が終わったら、`end_position`にchの長さを足し合わせていき、現在のposを把握しておく
- 最後に`mapping.flush()`で`run_info.text`にtext情報を格納していく
  - ここで、空白の除去が行われる。
- text情報を格納したら`start_position`に`end_position`を格納する

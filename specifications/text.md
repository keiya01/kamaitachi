# Text

## Servo Code Reading

- `UnscannedText`にはnodeから取り出したtextが含まれている
- `UnscannedText`に含まれているtextを`scan_for_runs`関数で分割し、それぞれのcharの情報を`RunInfo`に格納している
  - 格納する情報は、`font`, `script`(charの言語の種類), `bidi`など
  - `font`は一文字ごとにfontの種類が異なる可能性がある。例えば、`あBう`に`sans-serif`を指定している場合、`sans-serif`には日本語の情報がないため、`Hiragino Sans GB`を個別に適用する必要がある
  - `script`で言語別に`run_info`を分別していて、一つなぎの`katakana`なら同じ`run_info`の`text`に格納される。`script`が異なる場合、`run_info`を`run_info_list`に挿入し、`run_info`をflushする
  - これらの作業はとても重い作業であるため、読み込んだfontは必ずcacheする
- `transform_text`で文字列の空白を圧縮し、`run_info.text`に文字情報を格納する

### Simple Line Breaking

- `flush_clump_to_list()`で1文字づつchをみていき、その文字のfont周りのスタイルを取得していくが、`ch.is_control()`が`false`のときは、この処理を行わない
- スタイルは、bold(font-weight), italicなどを取得し、`run_info`に格納する
- chごとにfont-familyを確認していく。この時に`ucd::UnicodeBlock`を使って適用するfamilyを決定する
- textはscriptごとにまとめられていて、loopの中でscriptが異なっていれば、新しい`run_info`を作成する
- scriptは`unicode_script::Script::from`を使ってscriptを取得する(いったんfontごとに分類する感じでいいかも)
- スタイルの適用が終わったら、`end_position`にchの長さを足し合わせていき、現在のposを把握しておく
- 最後に`mapping.flush()`で`run_info.text`にtext情報を格納していく
  - ここで、空白の除去が行われる。
- text情報を格納したら`start_position`に`end_position`を格納する

### Beautiful Line Breaking

- `scan_runs`で`last_whitespace`を持っておき、`inline box`内のwhite spaceを削除する
- `range.start == 0`の場合は、改行しない。
- `xi_unicode`というcrateの`LineBreakLeafIter`を使って、禁則文字の判定をしている
  - `scan_runs`で`breaker`を`None`に指定し、`break_and_shape`で`breaker`に`LineBreakLeafIter::new()`を入れている
  - `breaker.next(text)`で一連のtextを引数に入れていくことで、全体のline breakの位置を取得している
  - `scan_runs`で初期化しておくことで、`break_at_zero`が`true`でかつ、`range.start == 0`である時に改行を抑制する
  - `word.char_indices().rev().take_while(|&(_, c)| char_is_whitespace(&c)).last()`の部分は、`char_is_whitespace`が`false`の場合には`None`が即座に返る

#### in calculate_split_position

- `text_fragment_info.range`にtext全体のrange情報が入っている
- `text_fragment_info.run.range`にglyph単位のsplit positionのrangeが入っている
- `natural_word_slices_in_range`では、`index_of_first_glyph_run_containing`に`text_fragment_info.range.begin()`を渡し、この`index`で始まるglyphを`text_fragment_info.run.glyphs`の中からbinary searchで探してその結果(text_runとindex、glyph)をキャッシュする
  - このglyphはline break opportunityごとにinstance化されている
- `calculate_split_position_using_breaking_strategy`では`slice_iterator`として、`natural_word_slices_in_range`の値を渡しており、`TextRunSlice`のIterator::next()を使ってglyphsの値を取り出しつつ、幅を計算している
  - glyphsにはglyph_storeが入っており、これはfont情報

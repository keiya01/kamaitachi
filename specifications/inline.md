# Inline Layout Module

CSS2: https://www.w3.org/TR/CSS21/box.html#box-model
CSS3: https://drafts.csswg.org/css-inline-3/

## Positioning

- `containing block`のtopから初めて水平方向に次々に配置される
- 水平方向のmargin, border, paddingが適用される
- 垂直方向の整列も可能
- lineを含む矩形は`line box`と呼ばれる
- `line box`のwidthは`containing block` or `float`の存在によって決まる
- `line box`の高さは[line-height](#line-height-calculation)によって決まる
- `line box`は含まれる全てのboxにとって十分な高さになる。(中身によって高さが変わる)
- When the height of a box B is less than the height of the line box containing it, the vertical alignment of B within the line box is determined by the 'vertical-align' property.(中身のコンテンツが`containing block`よりも小さい場合は`vertical-align`によって配置が決まる)
- 複数の`inline-level box`が一つの`line box`内に水平に配置できない場合、2つ以上の垂直に積み上げられた(`vertically-stacked`)`line box`に配分される
- 段落は垂直方向に積み上げられる。他の場所で指定されていない限り、重なったり、分離することはない。
- 一般的に`inline box`の`width`は同じですが、`float`により`width`が減少している場合、`width`が異なる場合がある
- `inline box`の`height`は一般的に異なります(例えば、一つのラインに`img`が含まれている場合)
- `inline-level box`の`width`の合計が`containing box`の`width`よりも小さい場合、`text-align`によって水平方向の配置が決まる
- もしpropertyが`justify`を持っていれば、スペースと文字を引き伸ばす(`inline-block`と`inline-table`はのぞく)
- `inline box`が`line box`の幅を超える時は、いくつかの`box`に区切って、それらの`box`を複数行に渡って`line box`を配置する
  - もし`inline box`が単一の単語を含んでいたり、言語の特定の改行ルールが無効になっていたり、`white-space`の値が`nowrap`または`pre`になっている場合は、`line box`をはみ出す
  - `line box`が分割される時、見た目に影響を与えません
  - `bidirectional text processing`のため、`inline box`は同じ`line box`内でいくつかの`box`に分割されるかもしれない
    - `bidirectional text processing` ... 双方向テキスト処理。ヘブライ語などの右から左に読むような言語をサポートする場合に必要。
- `line box`
  - `line box`は`inline formatting context`の内部で`inline-level content`を包含するために必要に応じて作られる
  - `line box`は`text, white-space, inline-element, non-zero margin, padding, border, img, inline-block`を含まない
        - 内部の要素の`positioning`の目的で使うため、zero-heightとして扱い、さらにその他の目的では存在していないものとして扱う

```html
<!-- line box example -->
<P>Several <EM>emphasized words</EM> appear
<STRONG>in this</STRONG> sentence, dear.</P>
```

上記の例では、`P`は5つの`inline box`を含む`block box`を生成している

- `Anonymous` ... "Several"
- `EM` ... "emphasized words"
- `Anonymous` ... "appear"
- `STRONG` ... "in this"
- `Anonymous` ... "sentence, dear."

これらをformatするために、`line box`を生成する。
今回の例では、P要素用に生成されたboxが`line box`のための`contain block`を確立する。もし`containing block`が十分に広い場合、全ての`inline box`は一つの`*line* box`に納る。

*Several emphasized words appear in this sentence, dear.*

もしそうでないなら、`inline box`は分割され、複数行にわたって配置される。これらの一行一行が`line box`であり、今回のケースでは2つの`line box`に分割されていることになる。

*Several emphasized words appear*  
*in this sentence, dear.*  

または次のようになる。

*Several emphasized*  
*words appear in this*  
*sentence, dear.*  

上記の例では、`EM box`が2つに分割されている(これらを`split1`, `split2`と呼ぶ)。
`margins`, `borders`, `padding`, または`text decorations`は`split1`の後または`split2`の前で視覚的な変化を起こさない。

つまり、
- `split1`は`margin`の`top`、`left`、にスタイルがあたり、`split2`は`margin`の`right`、`bottom`にスペースが当たる
- `split1`は`padding`と`border`の`top`、`left`、`bottom`にスタイルがあたり、`split2`は`margin`の`top`、`right`、`bottom`にスペースが当たる

## line height calculation

1. `replaced element`、`inline-block element`、`inline-table element`の場合、高さはmargin boxによって決まる。`inline box`の場合は、`line-height`によって決まる
2. `inline-level box`は`vertical-align`によって垂直方向に整列される。
3. `line box`の高さは、boxのtopとbottomの距離

#### Leading and Half Leading

- CSSは全てのfontは特徴的なbaselineの上の高さ(`Ascendent`)とbaselineの下の深さ(`Descendent`)を指定するfont metricsを持つと過程する
- Aを高さとし、Dを深さとした場合、`AD = A + D`と定義する。この距離はtopからbottomへの距離である。
- leadingを求めるには、`L = line-height - AD`とする
- さらに`A' = A - L / 2`, `D' = D - L / 2`とすることで合計の高さと深さを求められる。これらを足し合わせることで高さがもとまる。
- `A`と`D`は`OpenType`または`TrueType`のfontから`Ascendent`・`Descendent`を取得することで実装することができる。
  - `Ascendent`は文字の上半分、`Descendent`は文字の下半分のこと。[Java の Font 周りの比較的ディープな話(前編)](https://www.cresco.co.jp/blog/entry/91/)がわかりやすかった。
  - icedでは文字の左上を起点に座標を決めているっぽいのでこれらは必要なさそう？(単純に文字の高さを求めたい)
- `line-height`によって指定された高さが`containing box`よりも小さい場合、`background-color`などがはみ出る(これはまだよくわかってない)
> Although margins, borders, and padding of non-replaced elements do not enter into the line box calculation, they are still rendered around inline boxes. This means that if the height specified by 'line-height' is less than the content height of contained boxes, backgrounds and colors of padding and borders may "bleed" into adjoining line boxes. User agents should render the boxes in document order. This will cause the borders on subsequent lines to paint over the borders and text of previous lines.
- `line-height`の値には、line-boxの計算値が指定される

### Leadingの実装

- `servo/font-kit`の[Font::metrics(&self)](https://docs.rs/font-kit/0.10.0/font_kit/loaders/freetype/struct.Font.html#method.metrics)を使えば、`ascendent`と`descendent`を求められそう。
  - `iced`の実装例(https://github.com/hecrj/iced/blob/master/graphics/src/font/source.rs)
- またここで取得したfontデータは直接`iced_native::Text`に渡したい

### Calculating inline width

- `inline box`のユーザー定義の`width`は適用されない
- `margin-right`と`margin-left`の値は`0`になる

## Line Breaking

- `forced line break` ... 明示的に行の分割が操作されている、または、blockの始まり、または終わりによって分割されること
- `soft wrap break` ... コンテンツの折り返しによって行が分割された場合。例えば、測定しているbox内にコンテンツがfitしているために非強制的な行分割が行われている時。
- inline-levelのコンテンツを複数の行に分割する作業は行分割(`line breaking`)と呼ばれる
- 折り返しは、許可された分割ポイントでのみ行われる。これは[soft wrap opportunity](soft-wrap-opportunity)と呼ばれる。
- 折り返しが、[white-space](https://www.w3.org/TR/css-text-3/#propdef-white-space)によって有効になっている場合、`soft wrap opportunity`が存在するなら、ここで行を折り返すことによって、コンテンツがオーバーフローする量を最小化しなければならない。
- 句読点で行分割をすることをおすすめする
- 優先順位を適用するために、`containing block`の`width`、文字の言語、`line-break`の値、さらに他の要因を使う
- CSSは`line break opportunity`の優先順位を定義していない
- もし`word-break: break-all;`、`line-break: anywhere;`が指定されているなら、単語分割の優先順位は期待されない(どこでも改行できる)
- `out of flow`要素や`inline element`の境界では改行は起こらない
- たとえ通常はそれらを抑制するための文字(`NO-BREAK SPACE`)に隣接していたとしても、webの互換性のために、画像などのreplaced要素やその他のatomic inlineの前後で`soft wrap opportunity`がある
- 改行で消える文字によって作られる`soft wrap opportunity`の場合、その文字を直接含むボックスのプロパティは、それらの機会に改行を制御する(おそらく、overflowした段階で初めて改行が制御されるということ)
- 二つの文字の間の境界によって定義される`soft wrap opportunity`の場合、最も近い共通の祖先の`white-space`が分割を制御する。(`white-space`を参考に改行の方法を決める)
- `soft wrap opportunity`の前の最初と後の最後の文字のboxの場合、分割はboxのコンテンツの端とコンテンツの間というよりもむしろ、boxの前後(marginの端)ですぐに起こる
- [ruby boxの定義](https://www.w3.org/TR/css-ruby-1/#line-breaks)


### soft wrap opportunity

- 多くのWriting Systemではハイフネーション(英単語の途中で改行になった時に`-`で一続きの単語であることを意味すること)がない場合、単語の境界で`soft wrap opportunity`が起こる
- 多くのシステムでは、スペースや句読点をいくつかの単語を分割するために使う。そして、`soft wrap opportunity`はそれらの単語によって起こる
- タイやクメール語ではスペースや句読点を単語の分割のために使わない(**これはサポートしない**)
- いくつかの他のWriting Systemでは単語の境界ではなく、正書法の音節境界(?)をベースにしている
- 中国語や日本語では、いくつかの音節が一つの書体文の単位に対応する傾向がある。従って、行分割の慣習は特定の文字の間を除いて、どこでも分割することができる
- CSSでは`soft wrap opportunity`を区別するためのいくつかの方法を提供している
  - `line-break` ... 行分割の制限の様々な厳格なレベルを選ぶことができる
  - `word-break` ... 日本語や中国語のようなまとめられていて分割不可能な文を形成するために、CJK文字をnon-CJKのように扱う
  - `hyphens` ... ハイフネーションが行を分割することを許可される
  - `overflow-wrap` ... 溢れる可能性のある、分割不可能な文字を分割する

## Servo Code Reading

path: `servo/components/line.rs`

### Line Struct

```rs
pub struct Line {
    pub range: Range<FragmentIndex>,
    pub visual_runs: Option<Vec<(Range<FragmentIndex>, bidi::Level)>>, // TODO: for bidirectional
    pub bounds: LogicalRect<Au>, 
    pub green_zone: LogicalSize<Au>, // TODO: for float
    pub minimum_metrics: LineMetrics,
    pub metrics: LineMetrics,
}
```

- `Line` structは1つの`line box`を表している
- つまり、1つの`line box`のための改行の情報やwidthやheightなどのmetricsの情報を含みそれをもとに一行ごとのinline boxを表示していく
  
#### range field

type: `Range<FragmentIndex>`

```rs
pub struct Range<I> {
    begin: I,
    length: I,
}
``` 

```rs
int_range_index! {
    #[derive(Serialize)]
    #[doc = "The index of a fragment in a flattened vector of DOM elements."]
    struct FragmentIndex(isize)
}
```

- `FragmentIndex`はmacroでラップされている。このmacroではいろいろなtraitを実装しているが基本的な操作はstructから値を簡単に取り出せるようにしていたり、他のstructと足し合わせるような処理をしている。
- `range` fieldでは、改行の位置を示している。つまり`line box`がどこまでなのかを示すために使われる。

### bounds field

```rs
pub struct LogicalRect<T> {
    pub start: LogicalPoint<T>,
    pub size: LogicalSize<T>,
    debug_writing_mode: DebugWritingMode,
}
```

```rs
pub struct LogicalPoint<T> {
    /// inline-axis coordinate
    pub i: T,
    /// block-axis coordinate
    pub b: T,
    debug_writing_mode: DebugWritingMode,
}
```

```rs
pub struct LogicalSize<T> {
    pub inline: T, // inline-size, a.k.a. logical width, a.k.a. measure
    pub block: T,  // block-size, a.k.a. logical height, a.k.a. extent
    debug_writing_mode: DebugWritingMode,
}
```

- `LogicalPoint`は`line box`の正確なpositionを保持する
- `LogicalSize`は`line box`の拡張されたwidthやheightを保持する
  - 例えば、一つの`line box`上に画像などの大きい要素がはい位置された場合、高さはその高さに合わせられる
  - heightを`block`、widthを`inline`と呼ぶ

### minimum_metrics field

```rs
pub struct LineMetrics {
    pub space_above_baseline: Au,
    pub space_below_baseline: Au,
}
```

- これはstyleによって指定された`line-height`やfontに関するvisual情報を保持する

### metrics field

```rs
pub struct LineMetrics { ... }
```

- これは実際に計算された`line-height`やfontに関するvisual情報を保持する

### Inline Flow

- `impl LineBreaker`の`reflow_fragment`で改行周りの処理やpositionの計算をしている
- `fragment`に各nodeが入っていて、そのノードを分割して、InlineFlow.linesに入れている

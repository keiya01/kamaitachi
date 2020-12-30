# Visual Formatting Model

https://www.w3.org/TR/CSS22/visuren.html

## Visual Formatting Model とは

以下の要素によって決定される。

- `box dimensions` と `type`

- `positioning scheme`(normal flow, float, absolute positioning)
  - `normal flow` ... これはblock-levelのboxの`block formatting`, inline-levelのboxの`inline formatting`, block-level と inline-level の `relative positioning`を含むものである。
    - `block formatting`
      - `containing block`のleft-topから初めて、垂直方向に次々に配置される
      - 兄弟boxの間の垂直方向の距離はmarginによって決まる
      - 隣接する要素の垂直marginは相殺される(`margin collapse`)
      - floatが存在する場合でもleftから始まる
    - [inline formatting](./inline)
    - [ ] **TODO** [relative positioning](https://www.w3.org/TR/CSS22/visuren.html#relative-positioning)
  - [ ] **TODO** [float](https://www.w3.org/TR/CSS22/visuren.html#floats)
  - [ ] **TODO** [absolute positioning](https://www.w3.org/TR/CSS22/visuren.html#absolute-positioning)
- document tree での関係性
- その他の情報(viewport, 画像の`intrinsic dimension`)

もし要素が`float`されているか、absolute positioningであるまたは、root elementである場合は`out of flow`と呼ばれる。`out of flow`ではない要素は`in-flow`と呼ばれる。

## Controlling box generation

### Block-level elements

- 段落などのように視覚的にフォーマットされたドキュメントソースの要素
- `display`の値は、`block`, `list-item`, `table`である
- [block formatting context](#block-formatting-context)である。
- `block-level box`は`block container box`である。`block container box`は`block-level box`のみを含むか、[inline formatting context](#inline-formatting-context)を確立するため、[inline-level box](#anonymous-block-box)のみが含まれる。
- An element whose principal box is a block container box is a block container element. Values of the 'display' property which make a non-replaced element generate a block container include 'block', 'list-item' and 'inline-block'. Not all block container boxes are block-level boxes: non-replaced inline blocks and non-replaced table cells are block containers but are not block-level. Block-level boxes that are also block containers are called block boxes.(よくわかっていない)
  - `display`に`block`, `inline-block`などの値が指定された要素を`block container element`と呼ぶ
  - `non-replaced` elementを`block container`にすることもできるが、 `non-replaced inline blocks`と`non-replaced table cells`は`block-level`になることはできない
  - `block-level box`でかつ、`block container`である要素は`block box`と呼ばれる

#### block formatting context

- フロート、絶対配置要素、ブロックボックスではないブロックコンテナ（インラインブロック、表セル、表キャプションなど）、ブロックボックスではないブロックボックス、そして `visible` 以外の `overflow` を持つブロックボックス（その値がビューポートに伝搬されている場合を除く）は、そのcontentのために新しいブロック書式設定コンテキストを確立する

#### Anonymous block boxes

- もし、`block-level box`(下の例で`div`)のなかに`block-level box`(下の例で`p`)を持つ場合、formatを簡単にするために、inlineの要素("Some Text")を`Anonymous block box`で囲う
```html
<div style="display: block;">
  This is anonymous
  <p style="display: block;">More Text</p>
</div>
```

- `inline box`の間にin-flowの`block box`がある場合は、両端を分割して、`Anonymous block box`で囲う
```html
<div>
  <p>
    This is first anonymous <!-- C1 -->
    <span style="display: block;">This is span</span>
    This is second anonymous <!-- C2 -->
  </p>
</div>
```

- `Anonymous box`のstyleは囲われている`anonymous box`ではない要素から、継承される。(上記の例では、`p`のstyleが継承される)
- 継承されない値は`initial value`がある。例えば、`anonymous box`のfontは継承されるが、marginは0になる
- borderは周りに描画される(`C1`では終端は開き、`C2`では始端が開く)

### Inline-level elements

#### Anonymous inline boxes

- `block-level box`の中に`inline box`がある場合は、`anonymous inline box`で囲われる
- `Anonymous inline box`のstyleは親要素から、継承される
- 継承されない値は`initial value`を持つ。例えば`color`は`p`要素から継承されるが、`background`は`transparent`になる
- **TODO**: White space content that would subsequently be collapsed away according to the 'white-space' property does not generate any anonymous inline boxes.

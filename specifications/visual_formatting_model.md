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

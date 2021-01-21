# Font

## introduction

- fontは文字コードを文字のビジュアル表現を含むリソースを表す形状(glyph)にマップする情報を含んでいる
- 共通のデザインスタイルを共有するフォントは、`font-family`ごとにグループ化される
- familyないでは、stroke width, slant(傾斜) or relative widthなどによって異なる
- 個別のfont faceはこれらのプロパティのユニークな組み合わせによって説明される
- 特定の範囲のテキストに対して、CSSフォントプロパティは`font family`やレンダリングする時に使用されるfamily内の特定のfont faceを選択するために使用される
- local font resourceの場合、説明情報はfont resourceから直接取得できる

## typographic background

- 様々な種類の文体を持つfont faceのセットはfont familyにグループ化される。最もシンプルなケースでは、italicやboldが補充されるが、より広範なグループ化することもできる
- fontのcharacter mapはcharacterのmappingをこれらのfont用のglyphsに定義する
- もしドキュメントがfont family listに含まれるfontのcharacter mapによってサポートされていないcharacterを含んでいる場合、UAはより適切なfontを見つけるために`system font fallback`の手順を使用する。
- もし、適切なfontを見つけることができなかった場合、何らかの形で欠落したglyph文字がUAによってレンダリングされる
- `System fallback`は指定されているfont familyのリストが、与えられた文字をサポートしているfontを含んでいない場合に発生する

## Font family property

- 

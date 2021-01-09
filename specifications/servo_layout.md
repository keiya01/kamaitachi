# Entry Point

1. Entry Pointは`layout_thread/lib.rs`の[start](https://github.com/servo/servo/blob/master/components/layout_thread/lib.rs#L588)
2. eventのハンドリングをして、`reflow`である場合、`root_element`を取得し、`root_fragment`に格納する
3. その後、thread周りの処理をした後、[perform_post_style_recalc_layout_passes](https://github.com/servo/servo/blob/master/components/layout_thread/lib.rs#L1678)で`reflow`の処理をハンドリングしている
4. [マルチスレッドで処理するかどうか](https://github.com/servo/servo/blob/master/components/layout_thread/lib.rs#L1751-L1763)を決定した後、それぞれの方法で`reflow`を行う

## Sequential

ここでは逐次的に処理する場合の動作をまとめる

1. 逐次的に処理が行われる場合、[solve_constraints](https://github.com/servo/servo/blob/master/components/layout_thread/lib.rs#L976)で`sequential::reflow`が呼び出される
2. [sequential::reflow](https://github.com/servo/servo/blob/master/components/layout/sequential.rs#L30)の`doit`内部関数では、`AssignISizes`と`AssignBSizes`を受け取っている。
  - `AssignISizes`は`inline-size`の略である。`inline`とは横方向のスタイルである。
  - `AssignBSizes`は`block-size`の略である。`block`とは縦方向のスタイルである。
  - これらの`process`関数を呼び出すことでそれぞれのレイアウトを計算している
3. 
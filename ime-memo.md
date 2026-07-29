# wezterm IME・マルチバイト文字処理 調査メモ

ソースコード中の `[ime-memo]` でコメントを grep すると各所の詳細メモが見つかる。

```
rg '\[ime-memo\]' --type rust
```

## IME 入力の全体フロー

```
OS の IME イベント
  │  (プラットフォームごとの実装で 2 種類のイベントに正規化される)
  │
  ├─ 未確定文字列の更新 → WindowEvent::AdviseDeadKeyStatus(DeadKeyStatus::Composing(文字列))
  └─ 確定              → WindowEvent::KeyEvent(KeyCode::Composed(文字列))
  │
  ▼ wezterm-gui/src/termwindow/mod.rs
  ├─ AdviseDeadKeyStatus → self.dead_key_status に保持して再描画要求
  │     └─ 描画パス (render/pane.rs → render/screen_line.rs) が
  │        カーソル行の上に未確定文字列を overlay 描画(グリッドは触らない)
  └─ KeyEvent(Composed)  → keyevent.rs が UTF-8 でそのまま PTY へ write
        └─ シェルがエコーバック → term/src/terminalstate/performer.rs の
           flush_print() が grapheme 単位でグリッド(Screen)に格納
```

重要な非対称性: **未確定中は GUI が描く/確定後は端末出力として描く**。
2 経路の見え方・座標計算の差が IME バグの典型的な温床。

## プラットフォーム別 IME 実装

| OS | ファイル | 仕組み |
|---|---|---|
| Windows | `window/src/os/windows/window.rs` | IMM32 (WM_IME_COMPOSITION ほか)。TSF 不使用 |
| macOS | `window/src/os/macos/window.rs` | NSTextInputClient (set_marked_text / insert_text) |
| Wayland | `window/src/os/wayland/inputhandler.rs` | zwp_text_input_v3 (preedit/commit/done) |
| X11 | `window/src/os/x11/keyboard.rs` | xkb compose(デッドキー)。日本語 IME は XIM 非対応のこの版では ibus 等の実装に依存 |

## IME ウィンドウ(候補窓)の位置決め

- `termwindow/mod.rs` の `update_text_cursor()` がカーソルのセル座標を
  ピクセル矩形化 → `WindowOps::set_text_cursor_position()` で OS へ。
- Windows では `ime_preedit_rendering` 設定により
  `ImmSetCandidateWindow`(Builtin) / `ImmSetCompositionWindow`(System) を使い分け。
- ここで渡るのは「確定済みカーソル」の位置。未確定文字列の長さや折り返しは
  考慮されないため、行末付近では表示位置とずれることがある。

## 未確定文字列の overlay 描画(Builtin モード)

- `render/pane.rs`: `cursor.y == 行` かつアクティブペインのときだけ
  composing を quad キャッシュキー/シェイプキャッシュキーに載せる。
  → **カーソル行 1 行に限定**。行幅を超える未確定文字列は折り返されず見切れる
  (fix-cursor-position-for-ime-newline ブランチはここを行ごとのセグメントに拡張)。
- `render/screen_line.rs`: 行データを clone し
  `overlay_text_with_attribute(cursor.x, 未確定文字列)` で上書きしてからシェイプ。
  カーソル帯は `unicode_column_width(未確定文字列)` セルぶんに拡張。

## マルチバイト文字の基礎

- 格納は **grapheme(書記素クラスタ)単位**。1 Cell に 1 grapheme。
- 全角は「幅 2 の Cell 1 個 + 隠された後続桁 1 個」(`wezterm-surface/src/line/line.rs` の
  set_cell まわり)。後半桁への書き込みは全角文字ごと空白化される。
- 幅判定は `wezterm-cell/src/lib.rs` の `grapheme_column_width()`
  (Unicode 版依存。絵文字や曖昧幅で 1/2 が割れる)。
- 折り返しは `performer.rs` の deferred wrap(`x + width >= 右マージン` で
  wrap_next を立て、次の文字の印字時に改行)。

## 選択(マウス)とマルチバイト

- ピクセル→桁変換は `mouseevent.rs`(round による 50% 補正あり、全角非考慮)。
- 以降 `selection.rs` は終始「桁」単位。全角文字は 2 桁を占めるため、
  桁単位の ±1 補正が文字単位では半文字ズレになる
  (fix-multi-byte-chara-selection ブランチの領域)。
- コピー文字列化は `columns_as_str()`: 範囲始端が全角の後半桁だと
  その文字は含まれない。

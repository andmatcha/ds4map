# MANUALモード: コントローラー入力からACパケットまでの対応整理

このメモは、現行実装を読んで分かる範囲で、`MANUAL` モード時の

- コントローラー入力
- uplink packet (`MC`)
- ROSトピック
- 最終的な `AC v6` パケット

の対応関係を整理したものです。

## 先に要点

- `MANUAL` では、コントローラー入力はまず Mac 側 uplink の `MC` パケットとして送られます。
- Ubuntu 側 `udp_joy_bridge` が `MC` を受けて `/arm_ik/manual_currents` と `control_mode` / `control_byte` に分解します。
- `udp_ac_tx` はそれらを使って `AC v6` を再構成します。
- `MANUAL` では `AC v6` の `current[0..6]` が主役で、`angle[0..2]` / `vel[0..2]` は 0 です。
- `MANUAL` では `control_byte` のうち特に `NYOKKI_PUSH` / `NYOKKI_PULL` / `INIT` / `HOME` が関係します。

## データの流れ

通常経路:

1. コントローラー入力を `tools/gamepad_mode_sound.py` が読み取る
2. `MC` パケット `<2sBB7HB>` を UDP 5005 に送る
3. `src/arm_ik_control/arm_ik_control/udp_joy_bridge.py` が `MC` を受信する
4. `udp_joy_bridge` が `UInt16MultiArray` として `/arm_ik/manual_currents` を publish する
5. `udp_joy_bridge` が `control_mode` と `control_byte` も publish する
6. `src/arm_ik_control/arm_ik_control/udp_ac_tx.py` がそれらをまとめて `AC v6` を送る

関係ファイル:

- `tools/gamepad_mode_sound.py`
- `src/arm_ik_control/arm_ik_control/udp_joy_bridge.py`
- `src/arm_ik_control/arm_ik_control/udp_ac_tx.py`
- `docs/communication_data_formats.md`

## MANUAL時の `MC` パケット

`udp_joy_bridge.py` のコメント上、uplink packet は次の2種類です。

- `AI`: `<2sBBfffffB>`  (IK teleop)
- `MC`: `<2sBB7HB>`     (manual currents)

`MANUAL` で重要なのは `MC` です。

`MC` の内容:

- `header`
- `seq`
- `flags`
- `m1..m7`
- `control_byte`

ここで `m1..m7` が、最終的に `AC v6.current[0..6]` の元データになります。

## コントローラー入力 -> `MC.m1..m7` の対応

`tools/gamepad_mode_sound.py` の現行実装では、`MANUAL` 用電流値は次の対応です。

| MC field | AC field | 入力 | 備考 |
|---|---|---|---|
| `m1` | `current[0]` | `R2 / L2` | BaseHorizon |
| `m2` | `current[1]` | `R1 / L1` | BaseRoll |
| `m3` | `current[2]` | `Right Stick Y` | Pitch1 |
| `m4` | `current[3]` | `Left Stick Y` | Pitch2 |
| `m5` | `current[4]` | `Y / A` または `△ / ×` | Pitch3 |
| `m6` | `current[5]` | `B / X` または `○ / □` | Roll |
| `m7` | `current[6]` | `D-pad Right / Left` | Gripper |

実装上の対応ロジック:

- `R2 > 0.8` で `m1` 正方向、`L2 > 0.8` で `m1` 負方向
- `R1 == 1` で `m2` 正方向、`L1 == 1` で `m2` 負方向
- `Right Stick Y > 0.8` で `m3` 正方向、`< -0.8` で負方向
- `Left Stick Y > 0.8` で `m4` 正方向、`< -0.8` で負方向
- `Y` / `△` で `m5` 正方向、`A` / `×` で負方向
- `B` / `○` で `m6` 正方向、`X` / `□` で負方向
- `D-pad Right` で `m7` close、`D-pad Left` で `m7` open

注意:

- 実際に送る値は `255` を中立とする `0..511` の電流コマンドです。
- 正方向・負方向に入る具体値は `manual_constants.*` で決まります。
- そのため、「どの入力がどのチャンネルに対応するか」は固定ですが、「何アンプ相当の値を送るか」は設定プロファイルに依存します。

## `flags` のMANUALでの意味

`flags` は `gamepad_mode_sound.py` で生成され、`udp_joy_bridge.py` 経由で `udp_ac_tx.py` に届きます。

`AC v6` 側で意味がある主な bit は次です。

| Bit | 意味 | MANUALでの扱い |
|---:|---|---|
| 0 | enable | deadman / enable 状態 |
| 1 | gripper | MANUALでは基本使わない |
| 2 | mission_panel | `MANUAL` では立たない |
| 4-5 | control mode | `1` が `MANUAL` |

補足:

- `gamepad_mode_sound.py` は `MANUAL` 時、gripper は `current[6]` で操作するため、デジタルの `gripper` flag は使わないようにしています。
- `udp_ac_tx.py` では `flags |= (mode << 4)` で control mode を `AC` に入れています。

## `control_byte` のMANUALでの意味

`control_byte` は uplink から `udp_joy_bridge.py` に入り、そのまま `udp_ac_tx.py` に渡され、最終的に `AC v6` の byte30 に入ります。

共有定義上の bit:

| Bit | Name | MANUALでの主用途 |
|---:|---|---|
| 3 | `NYOKKI_PUSH` | nyokki push パルス |
| 4 | `NYOKKI_PULL` | nyokki pull パルス |
| 5 | `INIT` | initialize one-shot |
| 6 | `HOME` | home pose one-shot |

実装上、`MANUAL` に関係する主な入力は次です。

- `extra_nyokki_push_button` -> `NYOKKI_PUSH`
- `extra_nyokki_pull_button` -> `NYOKKI_PULL`
- `extra_initialize_button` -> `INIT`
- `extra_home_pose_button` -> `HOME`

補足:

- `README.md` にも `MANUAL nyokki` は `AC v6 control byte bit4 / bit3` と記載があります。
- `control_byte` は timeout を過ぎると `udp_ac_tx.py` 側で 0 に戻されます。
- UI からの `control_byte_override` があれば、`udp_joy_bridge.py` 側で uplink 値より優先される場合があります。

## ROSトピックへの分解

`udp_joy_bridge.py` は `MC` を受けると:

- `m1..m7` を `/arm_ik/manual_currents` に publish
- `flags` から control mode を解釈し `/arm_ik/control_mode` に publish
- `control_byte` を `/arm_ik/ac_control_byte` に publish

そのため、`udp_ac_tx.py` から見ると、`MANUAL` の `AC` 生成に必要なのは主に次です。

- `/arm_ik/manual_currents`
- `/arm_ik/control_mode`
- `/arm_ik/ac_control_byte`
- `/joy` 由来の enable 状態

## ROS入力 -> `AC v6` の対応

`udp_ac_tx.py` の `MANUAL` では、概ね次のように `AC v6` が組み立てられます。

| AC field | 値の由来 | MANUALでの値 |
|---|---|---|
| `header` | 固定 | `b"AC"` |
| `seq` | `udp_ac_tx` 内部カウンタ | 毎tickで加算 |
| `flags.bit0` | `/joy` の deadman と override | enable |
| `flags.bit1` | `/joy` の gripper | 実質未使用寄り |
| `flags.bit2` | IK submode | `MANUAL` では 0 |
| `flags.bit4-5` | `/arm_ik/control_mode` | `1` |
| `current[0..6]` | `/arm_ik/manual_currents` | `MC.m1..m7` 相当 |
| `angle[0..2]` | IK joint command | `0` |
| `vel[0..2]` | IK joint velocity | `0` |
| `control_byte` | `/arm_ik/ac_control_byte` | MANUAL用 bit 群 |
| `base_target_mm_j0` | keyboard auto 専用 | `0` |
| `auto_flags` | keyboard auto 専用 | fresh でなければ 0 |
| `fault_code` | keyboard auto 専用 | fresh でなければ 0 |
| `crc16` | `udp_ac_tx` で計算 | 自動付与 |

## MANUALで重要な注意点

### 1. enable が 0 だと current は全ch neutral に戻る

`udp_ac_tx.py` では `enable == 0` のとき、`current[0..6]` を全て neutral に戻します。

つまり、コントローラーで MANUAL の入力を入れていても、enable が落ちると `AC` 上は動作用電流になりません。

### 2. `MANUAL` では `angle` / `vel` は使わない

`udp_ac_tx.py` では `mode == MANUAL` の分岐で:

- `angle[0..2] = 0`
- `vel[0..2] = 0`
- `base_target_mm_j0 = 0`

に固定されます。

### 3. gripper は `flags.bit1` より `current[6]` が本体

MANUAL 時の gripper 操作は `D-pad Right / Left` から `current[6]` に変換される経路が主です。
そのため、IK のようなデジタル gripper flag 中心の見方とは少し違います。

### 4. ボタン番号そのものは設定で変わりうる

この文書で書いている `Y/A/B/X` や `R1/L1` などは、現行の `gamepad_mode_sound.py` のデフォルト解釈に基づくものです。

ただし実際には:

- Xbox系
- PS4系
- mapping JSON
- CLI引数

でボタン index は変えられます。

したがって、厳密には「物理ボタン番号」ではなく「論理入力名」が固定で、「その論理入力がどの物理ボタンに載るか」は設定依存です。

## ざっくり対応表

### 電流チャンネル

| コントローラー入力 | MC | AC v6 |
|---|---|---|
| `R2 / L2` | `m1` | `current[0]` |
| `R1 / L1` | `m2` | `current[1]` |
| `Right Stick Y` | `m3` | `current[2]` |
| `Left Stick Y` | `m4` | `current[3]` |
| `Y/A` または `△/×` | `m5` | `current[4]` |
| `B/X` または `○/□` | `m6` | `current[5]` |
| `D-pad Right/Left` | `m7` | `current[6]` |

### control byte

| 入力 | `control_byte` bit | AC v6 byte30 |
|---|---:|---|
| nyokki push button | 3 | `NYOKKI_PUSH` |
| nyokki pull button | 4 | `NYOKKI_PULL` |
| initialize button | 5 | `INIT` |
| home pose button | 6 | `HOME` |

## 根拠として読んだ箇所

- `tools/gamepad_mode_sound.py`
  - `compute_controls(...)`
  - `control_byte` の生成部
  - `motor1..motor7` の生成部
- `src/arm_ik_control/arm_ik_control/udp_joy_bridge.py`
  - `MC` unpack
  - `/arm_ik/manual_currents` publish
  - `control_mode` / `control_byte` publish
- `src/arm_ik_control/arm_ik_control/udp_ac_tx.py`
  - `/arm_ik/manual_currents` subscribe
  - `MANUAL` 分岐
  - `AC v6` pack
- `docs/communication_data_formats.md`
  - `AC v6` struct
  - `control_byte` bit 定義

## まだ曖昧な点

- `manual_constants.*` の具体値はプロファイルごとに異なるため、この文書では「どの入力がどのチャンネルに対応するか」を主眼にしています。
- UI override と gamepad uplink が同時に来た場合は、`control_mode` / `enable` / `control_byte` に override 優先ロジックが入るため、常に純粋なゲームパッド入力そのままとは限りません。
- `tools/gamepad_mode_sound_only_mac.py` / `tools/gamepad_mode_no_sound_only_mac.py` の Mac単体経路でも `AC v6` を直接作れますが、この文書は主に `gamepad_mode_sound.py -> udp_joy_bridge.py -> udp_ac_tx.py` の経路を対象にしています。

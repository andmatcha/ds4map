# コントローラー入力と最終 CAN 対応表

`docs/MANUAL_MODE_CONTROLLER_TO_AC.md` と `docs/received_packet_can_mapping.md` をもとに、`MANUAL` モードでの「コントローラー入力 -> 最終 CAN ID / DATA」の対応だけを抜き出した要約です。  
中間のパケットや ROS トピックは省略しています。

## 前提

- 対象は `MANUAL` モードです。
- 送信される CAN は 2 フレームです。
  - `0x200`: 前半 4 系統の電流
  - `0x201`: 後半 3 系統の電流 + `extra_flags`
- 16bit 値はすべてリトルエンディアンです。
  - 例: `Data[0] = LSB`, `Data[1] = MSB`

## 入力 -> CAN 対応

| コントローラー入力 | 意味 | CAN ID | DATA |
|---|---|---|---|
| `R2 / L2` | BaseHorizon | `0x200` | `Data[0..1]` = `current[0]` |
| `R1 / L1` | BaseRoll | `0x200` | `Data[2..3]` = `current[1]` |
| `Right Stick Y` | Joint1 | `0x200` | `Data[4..5]` = `current[2]` |
| `Left Stick Y` | Joint2 | `0x200` | `Data[6..7]` = `current[3]` |
| `Y / A` または `△ / ×` | Joint3 | `0x201` | `Data[0..1]` = `current[4]` |
| `B / X` または `○ / □` | Joint4 / Roll | `0x201` | `Data[2..3]` = `current[5]` |
| `D-pad Right / Left` | Gripper | `0x201` | `Data[4..5]` = `current[6]` |

## 補助ボタン -> `extra_flags`

`0x201` の `Data[6]` には `extra_flags` が入ります。  
`MANUAL` 系で使う主な対応は次の通りです。

| コントローラー入力 | `extra_flags` bit | CAN ID | DATA |
|---|---:|---|---|
| nyokki push button | 3 | `0x201` | `Data[6]` |
| nyokki pull button | 4 | `0x201` | `Data[6]` |
| initialize button | 5 | `0x201` | `Data[6]` |
| home pose button | 6 | `0x201` | `Data[6]` |

## CAN フレーム全体

### CAN ID `0x200`

| DATA byte | 内容 |
|---:|---|
| 0 | `current[0]` LSB |
| 1 | `current[0]` MSB |
| 2 | `current[1]` LSB |
| 3 | `current[1]` MSB |
| 4 | `current[2]` LSB |
| 5 | `current[2]` MSB |
| 6 | `current[3]` LSB |
| 7 | `current[3]` MSB |

### CAN ID `0x201`

| DATA byte | 内容 |
|---:|---|
| 0 | `current[4]` LSB |
| 1 | `current[4]` MSB |
| 2 | `current[5]` LSB |
| 3 | `current[5]` MSB |
| 4 | `current[6]` LSB |
| 5 | `current[6]` MSB |
| 6 | `extra_flags` |
| 7 | `0x00` |

## 注意

- 各入力の正方向 / 負方向で実際に入る数値そのものは設定値に依存します。
- この文書は「どの入力がどの CAN ID / DATA に載るか」だけを簡潔にまとめたものです。

# Tauri Weather Checker - Setup Complete ✅

## セットアップ完了状況

プロジェクトのスケルトンセットアップが正常に完了しました！

### 完了した項目

- ✅ Rust + Tauri プロジェクト構造の作成
- ✅ 全モジュールのスケルトン実装
  - `database.rs` - SQLite データベース操作
  - `config.rs` - 環境変数設定管理
  - `jma_feed.rs` - JMA XMLフィード取得（スケルトン）
  - `weather_checker.rs` - 警報チェックロジック
  - `notification.rs` - Gmail通知機能
  - `scheduler.rs` - 定期実行スケジューラー
  - `cleanup.rs` - データクリーンアップ
  - `error.rs` - エラーハンドリング
- ✅ 依存関係の設定（Cargo.toml）
- ✅ ビルド成功確認（`cargo check` 通過）
- ✅ 環境変数設定ファイル（.env.example）
- ✅ ドキュメント作成（README.md）

### プロジェクト構造

```
tauri-weather-checker/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # エントリーポイント
│   │   ├── config.rs         # 設定管理
│   │   ├── database.rs       # DB操作（完成）
│   │   ├── jma_feed.rs       # JMAフィード（要実装）
│   │   ├── weather_checker.rs # 警報チェック
│   │   ├── notification.rs   # メール通知（完成）
│   │   ├── cleanup.rs        # クリーンアップ（完成）
│   │   ├── scheduler.rs      # スケジューラー（完成）
│   │   └── error.rs          # エラー型
│   ├── Cargo.toml            # Rust依存関係
│   ├── tauri.conf.json       # Tauri設定
│   └── build.rs              # ビルドスクリプト
├── .env.example              # 環境変数テンプレート
├── .gitignore
└── README.md                 # プロジェクトドキュメント
```

## 次のステップ

### Phase 2: XML解析の実装（最優先）

`src-tauri/src/jma_feed.rs` の以下のメソッドを実装する必要があります:

1. **`parse_extra_xml`** - extra.xmlからVPWW54エントリを抽出
   ```rust
   pub async fn parse_extra_xml(&self, xml_content: &[u8]) -> Result<Vec<VPWWEntry>>
   ```

2. **`parse_vpww54`** - VPWW54形式のXMLをパース
   ```rust
   fn parse_vpww54(&self, xml_content: &str) -> Result<Vec<WarningData>>
   ```

3. **`get_latest_vpww54_for_lmo`** - 指定LMOの最新警報を取得
   ```rust
   pub async fn get_latest_vpww54_for_lmo(&self, lmo: &str) -> Result<Option<Vec<WarningData>>>
   ```

**参考:** Python実装の `app/JMAFeed.py` を見ながら実装してください。

### Phase 3: テストと統合

1. `.env` ファイルを作成（`.env.example` をコピー）
2. Gmail認証情報を設定
3. ローカルでビルド: `cargo build`
4. 実行してログ確認: `RUST_LOG=tauri_weather_checker=debug cargo run`
5. Python版と並行動作させて比較テスト

### Phase 4: システムトレイ（オプション）

`src-tauri/src/main.rs` でシステムトレイを有効化:
- アイコンを作成（適切な解像度のPNG）
- `Cargo.toml` に `features = ["tray-icon"]` を追加
- トレイメニューの実装

## 現在の制限事項

### 未実装機能

1. **JMA XMLパーサー** - 最重要！
   - extra.xml の解析
   - VPWW54フォーマットのパース
   - 都市別・警報種別の抽出

2. **システムトレイ**
   - 現在無効化（アイコン準備ができたら有効化可能）
   - メニュー機能（開始/停止、ログ表示、終了）

3. **GUI設定画面**（Phase 3以降）
   - メール設定
   - 監視地域の追加/削除
   - ログビューワー

### 既知の問題

- システムトレイは一時的に無効化されています（アイコン設定後に有効化可能）
- JMA XMLパーサーが未実装のため、実際の気象警報取得は動作しません
- 監視地域は `weather_checker.rs` にハードコード（後でGUIから設定可能に）

## ビルドとテスト

### ビルドチェック
```bash
cd src-tauri
cargo check
```

### デバッグ実行
```bash
cd src-tauri
RUST_LOG=tauri_weather_checker=debug cargo run
```

### リリースビルド
```bash
cd src-tauri
cargo build --release
```

実行ファイル: `src-tauri/target/release/tauri-weather-checker`

## 技術スタック

| 機能 | ライブラリ | バージョン |
|------|-----------|-----------|
| フレームワーク | Tauri | 2.1 |
| 非同期ランタイム | Tokio | 1.42 |
| スケジューラー | tokio-cron-scheduler | 0.13 |
| HTTP | reqwest | 0.12 |
| XML解析 | quick-xml, serde-xml-rs | 0.37, 0.6 |
| データベース | sqlx (SQLite) | 0.8 |
| メール | lettre | 0.11 |
| ログ | tracing | 0.1 |

## 環境変数

`.env` ファイルに以下を設定:

```env
# データディレクトリ
DATADIR=data/xml
DELETED_DIR=data/deleted
DB_PATH=data/weather.sqlite3

# Gmail設定
GMAIL_APP_PASS=your_gmail_app_password
GMAIL_FROM=your_email@gmail.com
EMAIL_TO=recipient@example.com
EMAIL_BCC=bcc@example.com  # オプション
```

## Python版との比較

| 項目 | Python + Docker | Rust + Tauri |
|------|----------------|--------------|
| メモリ使用 | ~100-200 MB | ~10-50 MB |
| 起動時間 | 3-5秒 | <1秒 |
| 配布サイズ | ~500 MB | ~5-15 MB |
| 依存関係 | Docker必須 | なし（単一バイナリ） |
| プラットフォーム | Docker対応OS | Windows/Mac/Linux ネイティブ |

## 次に実装すべきこと

**優先度：高**
1. JMA XML パーサーの実装（`jma_feed.rs`）
2. 実データでのテスト
3. エラーハンドリングの強化

**優先度：中**
4. システムトレイの有効化
5. ログファイル出力
6. 設定ファイルサポート

**優先度：低**
7. GUI設定画面
8. 通知履歴表示
9. 自動更新機能

## サポート

質問や問題があれば、以下を確認してください:

1. [README.md](README.md) - 詳細なドキュメント
2. [TAURI_MIGRATION_PLAN.md](../TAURI_MIGRATION_PLAN.md) - 移行計画全体
3. Python実装 (`../app/`) - リファレンス実装

---

**セットアップ完了日:** 2026-01-11
**ステータス:** Phase 1 完了 ✅ → Phase 2 へ進む準備完了

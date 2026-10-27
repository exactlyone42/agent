# 原始紀錄保存

首次整批提交時，Git 空白檢查拒絕測試紀錄與差異檔的行尾空白。文字副本只移除行尾空白及檔尾多餘空行；原始位元組完整保存在 [whitespace-originals.zip](whitespace-originals.zip)，內含 sha256.json 並逐檔核對。需要重套差異檔或核對原始輸出時，請使用封存原件。程式碼未改動。

後續 Prettier 格式整理前的原件另存 [formatting-originals.zip](formatting-originals.zip)，含逐檔 SHA-256。需要核對原始 JSON 排版或 AI 審查原文時使用封存檔；文字副本由既有格式化工具整理。

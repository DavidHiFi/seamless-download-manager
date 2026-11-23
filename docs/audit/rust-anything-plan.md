# Rust migration plan: ab-download-manager (Kotlin)

24 packages, 1071 source files (93920 lines), 0 test files, 0 UI files, grouped into 304 units.

| Ready | Caution | Blocked | Keep in source language |
| ---: | ---: | ---: | ---: |
| 105 | 50 | 149 | 0 |

> 23 files had parse errors; their units are marked caution.

## Start here

Best first ports: safe to port (readiness) and worth porting (loops and string work, tests, size).

| # | Unit | Lines | Priority | Readiness | Tests | Bridge | Main risks |
| ---: | --- | ---: | ---: | ---: | ---: | --- | --- |
| 1 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/` | 137 | 52 | 78 | 0 | JNI | String offsets and slicing (1); Null references (5); Integer width and overflow (3) |
| 2 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/` | 46 | 48 | 85 | 0 | JNI | Class inheritance (1); Concurrency (1) |
| 3 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` | 235 | 44 | 67 | 0 | JNI | Null references (8); Class inheritance (3); Integer width and overflow (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/` | 284 | 43 | 64 | 0 | JNI | Dynamic typing (4); Class inheritance (4); Concurrency (1) |
| 5 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/` | 371 | 30 | 38 | 0 | JNI | Class inheritance (8); Concurrency (5); Dynamic typing (2) |
| 6 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/` | 596 | 13 | 2 | 0 | JNI | String offsets and slicing (3); Null references (23); Concurrency (10) |
| 7 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/` | 1318 | 5 | 0 | 0 | JNI | Callback-shaped APIs (31); Null references (10); Dynamic typing (8) |

### `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/`

137 lines in 2 files, wave 0, priority 52, readiness 78, bridge **JNI**.

- 1 Maven dependencies not in the crate table: check them by hand.
- Leaf: imports no other unit.
- No tests found: write characterization tests against the original code first.
- String offsets and slicing x1: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/fileNameExtractor.kt:80`
- Null references x5: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:6`, `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:7`, `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:10`
- Integer width and overflow x3: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:26`, `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:26`, `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:27`
- Exceptions x1: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/RangeHeaderExtractor.kt:24`

### `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/`

46 lines in 2 files, wave 4, priority 48, readiness 85, bridge **JNI**.

- 2 Maven dependencies not in the crate table: check them by hand.
- Import cycle with desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/: port them together.
- No tests found: write characterization tests against the original code first.
- Class inheritance x1: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/Cli.kt:19`
- Concurrency x1: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/CliApp.kt:4`

### `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/`

235 lines in 4 files, wave 2, priority 44, readiness 67, bridge **JNI**.

- 1 parse errors: results for this unit are partial.
- 3 Maven dependencies not in the crate table: check them by hand.
- No tests found: write characterization tests against the original code first.
- Null references x8: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/BaseAppSettingsStorage.kt:18`, `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/BaseAppSettingsStorage.kt:19`, `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/DefaultAppSettings.kt:15`
- Class inheritance x3: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/BaseAppSettingsStorage.kt:12`, `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/DefaultAppSettings.kt:9`, `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/IAppSettingsModel.kt:7`
- Integer width and overflow x1: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/DefaultAppSettings.kt:31`
- Concurrency x1: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/BaseAppSettingsStorage.kt:8`

### `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/`

284 lines in 4 files, wave 4, priority 43, readiness 64, bridge **JNI**.

- 1 parse errors: results for this unit are partial.
- 2 Maven dependencies not in the crate table: check them by hand.
- Import cycle with desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/: port them together.
- No tests found: write characterization tests against the original code first.
- Dynamic typing x4: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:46`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:67`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:74`
- Class inheritance x4: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/DefinedHandlers.kt:11`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/DefinedHandlers.kt:29`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:22`
- Concurrency x1: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:9`
- Null references x1: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/BrowserMessageRequest.kt:22`
- Exceptions x9: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:37`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:64`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:68`

### `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/`

371 lines in 6 files, wave 4, priority 30, readiness 38, bridge **JNI**.

- 4 Maven dependencies not in the crate table: check them by hand.
- Import cycle with desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/: port them together.
- No tests found: write characterization tests against the original code first.
- Class inheritance x8: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:91`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:92`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:93`
- Concurrency x5: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:7`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:8`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt:8`
- Dynamic typing x2: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:96`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/EnsureAppIsAwake.kt:96`
- Division and modulo x2: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceManager.kt:8`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceManager.kt:11`
- Null references x1: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt:23`
- Exceptions x2: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleAppInstanceLocker.kt:40`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt:72`

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

### 1. android/app (`android/app`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/` | blocked | 136 | 57 | JNI | Division and modulo (1); Null references (1) |
| 0 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/page/` | blocked | 270 | 54 | JNI | Division and modulo (3); Null references (1) |
| 0 | `android/app/src/main/kotlin/com/abdownloadmanager/android/util/compose/` | blocked | 45 | 52 | JNI | Concurrency (1); Null references (1) |
| 0 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/widget/` | blocked | 683 | 0 | JNI | Null references (37); Dynamic typing (23) |
| 1 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/initialsetup/` | blocked | 150 | 69 | JNI | Class inheritance (1) |
| 1 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/about/` | blocked | 406 | 62 | JNI | Division and modulo (3); Integer width and overflow (1) |
| 1 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/perhostsettings/` | blocked | 372 | 33 | JNI | Division and modulo (4); Class inheritance (3) |
| 1 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/permissions/` | blocked | 725 | 0 | JNI | String offsets and slicing (2); Class inheritance (13) |
| 2 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/item/` | ready | 27 | 80 | JNI | Class inheritance (2); Concurrency (2) |
| 3 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/` | ready | 14 | 92 | JNI | Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/` | ready | 44 | 93 | JNI | - |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/action/` | caution | 17 | 90 | JNI | - |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/repository/` | ready | 46 | 80 | JNI | Concurrency (5); Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/` | blocked | 19 | 76 | JNI | Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/` | blocked | 30 | 75 | JNI | - |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/` | blocked | 242 | 67 | JNI | Callback-shaped APIs (2); Division and modulo (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/` | blocked | 42 | 66 | JNI | Class inheritance (1); Null references (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/` | blocked | 43 | 60 | JNI | Class inheritance (1); Concurrency (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/` | blocked | 90 | 59 | JNI | Callback-shaped APIs (2); Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/` | blocked | 270 | 57 | JNI | Class inheritance (2); Null references (2) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/` | blocked | 344 | 56 | JNI | Division and modulo (6) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/` | blocked | 318 | 55 | JNI | Division and modulo (4); Callback-shaped APIs (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/` | blocked | 356 | 55 | JNI | Dynamic typing (3); Null references (3) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/` | blocked | 341 | 54 | JNI | Dynamic typing (3); Class inheritance (2) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/` | blocked | 424 | 54 | JNI | Null references (5); Division and modulo (2) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/` | blocked | 218 | 54 | JNI | Class inheritance (5); Division and modulo (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/` | blocked | 302 | 53 | JNI | Callback-shaped APIs (9); Concurrency (2) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/` | blocked | 505 | 48 | JNI | Division and modulo (4); Callback-shaped APIs (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/` | blocked | 211 | 42 | JNI | Reflection and prototypes (2); Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/` | blocked | 530 | 42 | JNI | Null references (6); Callback-shaped APIs (5) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/` | blocked | 733 | 41 | JNI | Null references (5); Division and modulo (4) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/` | blocked | 324 | 40 | JNI | Concurrency (3); Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/` | blocked | 652 | 33 | JNI | Null references (9); Callback-shaped APIs (7) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/storage/` | blocked | 160 | 33 | JNI | Class inheritance (8); Callback-shaped APIs (3) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/` | blocked | 251 | 31 | JNI | Null references (2); Class inheritance (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/` | blocked | 603 | 30 | JNI | Division and modulo (5); Class inheritance (4) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/` | blocked | 466 | 30 | JNI | Null references (5); Callback-shaped APIs (3) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/` | blocked | 590 | 30 | JNI | Callback-shaped APIs (5); Division and modulo (4) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/di/` | blocked | 717 | 18 | JNI | Reflection and prototypes (2); Concurrency (1) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/service/` | blocked | 106 | 11 | JNI | Class inheritance (5); Concurrency (3) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/` | blocked | 753 | 0 | JNI | Callback-shaped APIs (6); Division and modulo (3) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/` | blocked | 992 | 0 | JNI | Callback-shaped APIs (9); Dynamic typing (6) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/` | blocked | 1544 | 0 | JNI | String offsets and slicing (7); Null references (32) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/` | blocked | 523 | 0 | JNI | String offsets and slicing (3); Null references (12) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/` | blocked | 3039 | 0 | JNI | String offsets and slicing (5); Dynamic typing (43) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/` | blocked | 813 | 0 | JNI | Concurrency (12); Division and modulo (8) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/` | blocked | 1142 | 0 | JNI | Dynamic typing (19); Division and modulo (13) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/` | blocked | 1369 | 0 | JNI | Class inheritance (22); Callback-shaped APIs (20) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/` | blocked | 2406 | 0 | JNI | String offsets and slicing (2); Callback-shaped APIs (20) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/util/` | blocked | 1115 | 0 | JNI | Concurrency (19); Null references (14) |
| 4 | `android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/` | blocked | 194 | 0 | JNI | Class inheritance (4); Null references (3) |
| 5 | `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/` | blocked | 79 | 52 | JNI | Class inheritance (1); Reflection and prototypes (1) |

### 2. buildSrc (`buildSrc`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `buildSrc/src/main/kotlin/Plugins.kt` | ready | 35 | 95 | JNI | - |
| 0 | `buildSrc/src/main/kotlin/buildlogic/versioning/` | caution | 37 | 83 | JNI | Dynamic typing (1); Null references (1) |
| 2 | `buildSrc/src/main/kotlin/buildlogic/` | ready | 184 | 76 | JNI | String offsets and slicing (1); Null references (2) |

### 3. compositeBuilds/plugins/common-android (`compositeBuilds/plugins/common-android`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `compositeBuilds/plugins/common-android/src/main/kotlin/ir/amirab/plugin/common_android/task/` | caution | 229 | 54 | JNI | String offsets and slicing (3); Class inheritance (4) |

### 4. compositeBuilds/plugins/git-version-plugin (`compositeBuilds/plugins/git-version-plugin`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `compositeBuilds/plugins/git-version-plugin/src/main/kotlin/ir/amirab/git_version/core/` | caution | 340 | 46 | JNI | Null references (41); Class inheritance (11) |
| 1 | `compositeBuilds/plugins/git-version-plugin/src/main/kotlin/ir/amirab/git_version/` | ready | 13 | 94 | JNI | Class inheritance (1) |

### 5. compositeBuilds/plugins/installer-plugin (`compositeBuilds/plugins/installer-plugin`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/utils/` | ready | 4 | 97 | JNI | - |
| 0 | `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/tasks/macos/` | ready | 116 | 89 | JNI | Class inheritance (1); Exceptions (1) |
| 1 | `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/tasks/windows/` | ready | 83 | 86 | JNI | Class inheritance (1) |
| 1 | `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/` | ready | 105 | 82 | JNI | Class inheritance (1); Reflection and prototypes (1) |
| 1 | `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/extensiion/` | ready | 115 | 74 | JNI | Null references (19); Class inheritance (2) |

### 6. compositeBuilds/shared/platform (`compositeBuilds/shared/platform`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `compositeBuilds/shared/platform/src/commonMain/kotlin/ir/amirab/util/platform/` | caution | 137 | 78 | JNI | Class inheritance (13); Null references (3) |

### 7. desktop/app (`desktop/app`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/template/` | blocked | 58 | 78 | JNI | Division and modulo (2) |
| 0 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/` | blocked | 108 | 74 | JNI | - |
| 0 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/renderapi/` | ready | 104 | 71 | JNI | Null references (6); Class inheritance (1) |
| 0 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/proxy/` | ready | 84 | 70 | JNI | Concurrency (4); Null references (4) |
| 0 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/util/` | blocked | 62 | 68 | JNI | Null references (7) |
| 2 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/repository/` | ready | 72 | 72 | JNI | Callback-shaped APIs (2); Concurrency (2) |
| 2 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/` | blocked | 2237 | 0 | JNI | String offsets and slicing (2); Class inheritance (17) |
| 3 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/` | ready | 38 | 94 | JNI | - |
| 3 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/updater/` | blocked | 348 | 56 | JNI | Division and modulo (6) |
| 3 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/` | blocked | 432 | 49 | JNI | Null references (4); Division and modulo (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/` | ready | 15 | 96 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/` | ready | 15 | 96 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/` | ready | 22 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/` | ready | 16 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/` | ready | 16 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/` | ready | 17 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/` | ready | 18 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/` | ready | 14 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/` | ready | 19 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/` | ready | 13 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/` | ready | 14 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/` | ready | 15 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/` | ready | 20 | 92 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/` | ready | 26 | 91 | JNI | Class inheritance (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/` | ready | 31 | 91 | JNI | Class inheritance (1); Exceptions (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/` | ready | 21 | 88 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/` | ready | 114 | 88 | JNI | Integer width and overflow (2); Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/` | ready | 46 | 85 | JNI | Class inheritance (1); Concurrency (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/` | ready | 63 | 85 | JNI | Class inheritance (1); Exceptions (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/` | ready | 103 | 84 | JNI | Callback-shaped APIs (4); Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/` | ready | 31 | 80 | JNI | Class inheritance (2); Concurrency (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/` | ready | 80 | 79 | JNI | Class inheritance (1); Concurrency (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/` | blocked | 154 | 79 | JNI | - |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/` | blocked | 22 | 78 | JNI | - |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/` | ready | 106 | 76 | JNI | Class inheritance (5); Callback-shaped APIs (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/` | blocked | 115 | 76 | JNI | - |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/` | blocked | 62 | 75 | JNI | Class inheritance (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/` | caution | 150 | 73 | JNI | Concurrency (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/` | blocked | 124 | 72 | JNI | Dynamic typing (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/` | blocked | 161 | 70 | JNI | Integer width and overflow (1); Division and modulo (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/` | caution | 284 | 64 | JNI | Dynamic typing (4); Class inheritance (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/` | ready | 125 | 62 | JNI | String offsets and slicing (3); Dynamic typing (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/` | blocked | 316 | 62 | JNI | Callback-shaped APIs (2); Division and modulo (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/` | blocked | 452 | 60 | JNI | Division and modulo (9) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/` | ready | 206 | 60 | JNI | Dynamic typing (5); Concurrency (3) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/` | blocked | 231 | 58 | JNI | Concurrency (3); Dynamic typing (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/` | blocked | 340 | 56 | JNI | Dynamic typing (4); Class inheritance (3) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/` | blocked | 302 | 56 | JNI | Null references (4); Callback-shaped APIs (2) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/` | ready | 270 | 53 | JNI | Callback-shaped APIs (8); Class inheritance (5) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/` | blocked | 340 | 50 | JNI | Class inheritance (4); Division and modulo (3) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/` | blocked | 413 | 49 | JNI | Dynamic typing (10); Class inheritance (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/` | blocked | 320 | 45 | JNI | Division and modulo (6); Class inheritance (3) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/` | blocked | 189 | 42 | JNI | Class inheritance (7); Null references (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/` | caution | 738 | 40 | JNI | Reflection and prototypes (2); Concurrency (1) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/` | blocked | 713 | 40 | JNI | Class inheritance (6); Callback-shaped APIs (3) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/` | blocked | 418 | 40 | JNI | Class inheritance (4); Division and modulo (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/` | caution | 371 | 38 | JNI | Class inheritance (8); Concurrency (5) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/` | blocked | 330 | 35 | JNI | Concurrency (7); Division and modulo (5) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/` | blocked | 1017 | 34 | JNI | Division and modulo (11); Null references (7) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/` | blocked | 585 | 34 | JNI | Callback-shaped APIs (5); Null references (5) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/` | blocked | 455 | 32 | JNI | Callback-shaped APIs (5); Class inheritance (4) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/` | blocked | 614 | 25 | JNI | Class inheritance (10); Dynamic typing (5) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/` | blocked | 681 | 21 | JNI | String offsets and slicing (2); Null references (6) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/` | blocked | 810 | 8 | JNI | Class inheritance (8); Callback-shaped APIs (7) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/` | blocked | 756 | 7 | JNI | String offsets and slicing (1); Null references (16) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/` | blocked | 1204 | 4 | JNI | Dynamic typing (13); Class inheritance (11) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/` | caution | 1318 | 0 | JNI | Callback-shaped APIs (31); Null references (10) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/` | blocked | 1732 | 0 | JNI | Dynamic typing (16); Null references (16) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/` | blocked | 979 | 0 | JNI | String offsets and slicing (7); Callback-shaped APIs (24) |
| 4 | `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/` | blocked | 1401 | 0 | JNI | Dynamic typing (28); Division and modulo (11) |

### 8. desktop/app-utils (`desktop/app-utils`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/` | ready | 56 | 74 | JNI | Hash iteration order (1); Callback-shaped APIs (1) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/` | blocked | 222 | 62 | JNI | Null references (12); Class inheritance (1) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/` | blocked | 231 | 59 | JNI | Null references (19) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/` | blocked | 334 | 57 | JNI | Null references (17); Class inheritance (1) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/` | blocked | 248 | 48 | JNI | Integer width and overflow (6); Callback-shaped APIs (5) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/` | blocked | 325 | 43 | JNI | String offsets and slicing (4); Null references (20) |
| 2 | `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/` | blocked | 976 | 0 | JNI | Null references (34); Concurrency (14) |

### 9. desktop/mac_utils (`desktop/mac_utils`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `desktop/mac_utils/src/main/kotlin/ir/amirab/util/desktop/mac/native/` | ready | 4 | 98 | JNI | - |
| 0 | `desktop/mac_utils/src/main/kotlin/ir/amirab/util/desktop/mac/event/` | ready | 39 | 91 | JNI | Class inheritance (1); Callback-shaped APIs (1) |

### 10. desktop/shared (`desktop/shared`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/poweraction/` | ready | 81 | 71 | JNI | Class inheritance (3); Callback-shaped APIs (1) |
| 0 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/screen/` | blocked | 66 | 55 | JNI | Integer width and overflow (18); Division and modulo (2) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/activator/mac/` | ready | 15 | 93 | JNI | Reflection and prototypes (2); Class inheritance (1) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/linux/` | ready | 33 | 92 | JNI | Class inheritance (1) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/windows/` | ready | 26 | 92 | JNI | Class inheritance (1) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/dock/mac/` | ready | 36 | 91 | JNI | Reflection and prototypes (4); Class inheritance (1) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/mac/` | ready | 40 | 82 | JNI | Reflection and prototypes (4); Class inheritance (2) |
| 1 | `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/` | blocked | 227 | 41 | JNI | String offsets and slicing (4); Class inheritance (5) |

### 11. desktop/slf4j-impl (`desktop/slf4j-impl`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `desktop/slf4j-impl/src/main/kotlin/ir/amirab/util/logger/slf4j/impl/` | ready | 102 | 75 | JNI | Null references (11); Class inheritance (2) |

### 12. downloader/core (`downloader/core`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/anntation/` | ready | 7 | 100 | JNI | - |
| 0 | `downloader/core/src/androidMain/kotlin/ir/amirab/downloader/utils/` | ready | 28 | 97 | JNI | Class inheritance (1) |
| 0 | `downloader/core/src/desktopMain/kotlin/ir/amirab/downloader/utils/` | ready | 67 | 93 | JNI | Null references (2); Class inheritance (1) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/contexts/` | ready | 35 | 88 | JNI | Class inheritance (11) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/proxy/` | ready | 63 | 87 | JNI | Class inheritance (6); Null references (6) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/` | caution | 87 | 86 | JNI | Null references (4); Class inheritance (1) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/exception/` | ready | 113 | 79 | JNI | Class inheritance (12); Null references (4) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/` | ready | 137 | 78 | JNI | String offsets and slicing (1); Null references (5) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/utils/speedlimiter/` | ready | 162 | 63 | JNI | Integer width and overflow (4); Class inheritance (2) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/` | ready | 294 | 55 | JNI | Null references (17); Callback-shaped APIs (8) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/` | caution | 351 | 50 | JNI | Null references (19); Class inheritance (11) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/` | caution | 492 | 46 | JNI | Concurrency (12); Class inheritance (8) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/db/` | caution | 397 | 36 | JNI | String offsets and slicing (11); Concurrency (12) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/destination/` | caution | 484 | 34 | JNI | Concurrency (18); Null references (11) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/utils/` | caution | 410 | 19 | JNI | String offsets and slicing (4); Hash iteration order (1) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/http/` | caution | 969 | 18 | JNI | Hash iteration order (1); Null references (40) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/` | caution | 749 | 13 | JNI | Concurrency (36); Class inheritance (14) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/queue/` | caution | 1132 | 13 | JNI | String offsets and slicing (18); Dynamic typing (14) |
| 0 | `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/hls/` | caution | 1025 | 8 | JNI | Hash iteration order (1); Null references (39) |

### 13. downloader/monitor (`downloader/monitor`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/` | blocked | 596 | 20 | JNI | Null references (21); Dynamic typing (19) |

### 14. integration/server (`integration/server`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `integration/server/src/main/kotlin/com/abdownloadmanager/integration/model/` | ready | 73 | 74 | JNI | Null references (12); Class inheritance (2) |
| 1 | `integration/server/src/main/kotlin/com/abdownloadmanager/integration/` | ready | 209 | 68 | JNI | Callback-shaped APIs (5); Class inheritance (4) |

### 15. shared/app (`shared/app`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/credits/translators/` | ready | 17 | 100 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/extractors/` | ready | 4 | 100 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/` | caution | 9 | 98 | JNI | Concurrency (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` | caution | 54 | 97 | JNI | Concurrency (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/di/` | ready | 3 | 96 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` | caution | 8 | 96 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/schemakt/` | caution | 14 | 96 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/sort/` | caution | 64 | 94 | JNI | - |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/` | caution | 41 | 93 | JNI | Concurrency (3); Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/appinfo/` | ready | 26 | 92 | JNI | Null references (2) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/notification/` | caution | 42 | 89 | JNI | Class inheritance (1); Callback-shaped APIs (1) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` | blocked | 6 | 88 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/notification/` | caution | 53 | 88 | JNI | Dynamic typing (1); Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/icon/` | ready | 91 | 88 | JNI | Class inheritance (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/util/` | blocked | 7 | 88 | JNI | - |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` | blocked | 6 | 88 | JNI | - |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/` | blocked | 36 | 87 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/category/` | ready | 57 | 87 | JNI | Class inheritance (2) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` | blocked | 33 | 87 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/updater/` | ready | 77 | 85 | JNI | Concurrency (2); Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/proxy/` | ready | 152 | 85 | JNI | Class inheritance (2); Concurrency (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/` | blocked | 41 | 85 | JNI | Concurrency (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pagemanager/` | ready | 92 | 84 | JNI | Null references (2); Class inheritance (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/styled/` | blocked | 38 | 83 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` | blocked | 14 | 82 | JNI | Class inheritance (6) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` | caution | 57 | 82 | JNI | Concurrency (8); Class inheritance (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` | caution | 80 | 82 | JNI | Concurrency (4); Class inheritance (2) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` | blocked | 17 | 81 | JNI | Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderror/` | ready | 39 | 81 | JNI | Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ondownloadcompletion/` | ready | 57 | 81 | JNI | Concurrency (7); Callback-shaped APIs (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/` | blocked | 40 | 79 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/perhostsettings/` | caution | 50 | 79 | JNI | Null references (7); Concurrency (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/` | caution | 248 | 77 | JNI | Class inheritance (2); Concurrency (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/shared/` | blocked | 34 | 77 | JNI | Null references (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/onqueuecompletion/` | ready | 63 | 77 | JNI | Concurrency (5); Dynamic typing (3) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` | blocked | 7 | 76 | JNI | - |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/faileddownloads/` | ready | 72 | 76 | JNI | Dynamic typing (7); Concurrency (6) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` | blocked | 86 | 75 | JNI | Null references (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/action/` | caution | 376 | 75 | JNI | Concurrency (6); Callback-shaped APIs (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` | caution | 89 | 74 | JNI | Null references (6); Class inheritance (4) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/theme/` | blocked | 75 | 73 | JNI | Division and modulo (2); Null references (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/extractors/linkextractor/` | ready | 159 | 73 | JNI | Class inheritance (6); Callback-shaped APIs (5) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` | blocked | 145 | 73 | JNI | Null references (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/edit/` | ready | 280 | 72 | JNI | Concurrency (8); Class inheritance (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/repository/` | ready | 201 | 72 | JNI | Concurrency (10); Callback-shaped APIs (6) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/mvi/` | blocked | 97 | 72 | JNI | Concurrency (12); Class inheritance (6) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/` | ready | 56 | 72 | JNI | Concurrency (4); Class inheritance (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/edit/` | ready | 293 | 71 | JNI | Concurrency (9); Class inheritance (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/impl/` | blocked | 45 | 71 | JNI | Class inheritance (3); Concurrency (3) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/add/` | caution | 226 | 69 | JNI | Concurrency (4); Null references (3) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/queue/` | blocked | 123 | 69 | JNI | Concurrency (3); Null references (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/autoremove/` | ready | 156 | 69 | JNI | Class inheritance (4); Callback-shaped APIs (3) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` | blocked | 246 | 69 | JNI | Null references (4) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/enterurl/` | ready | 111 | 68 | JNI | Class inheritance (5); Concurrency (4) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/category/` | ready | 136 | 67 | JNI | Concurrency (3); Null references (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/` | blocked | 92 | 67 | JNI | Null references (3); Dynamic typing (1) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/native/` | blocked | 94 | 67 | JNI | Dynamic typing (3); Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/batchdownload/` | ready | 161 | 65 | JNI | Class inheritance (10); Concurrency (6) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/editdownload/` | ready | 95 | 65 | JNI | Null references (4); Callback-shaped APIs (3) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/theme/` | blocked | 125 | 63 | JNI | Division and modulo (2); Dynamic typing (1) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/` | blocked | 35 | 62 | JNI | Class inheritance (2); Concurrency (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/add/` | caution | 253 | 62 | JNI | Concurrency (7); Null references (7) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/` | ready | 184 | 61 | JNI | Null references (8); Class inheritance (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/addToQueue/` | blocked | 75 | 61 | JNI | Null references (4); Callback-shaped APIs (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/edit/` | ready | 315 | 60 | JNI | Concurrency (18); Class inheritance (9) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/dns/` | caution | 92 | 60 | JNI | Class inheritance (3); Dynamic typing (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/definederrors/` | ready | 421 | 59 | JNI | Dynamic typing (17); Class inheritance (12) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/settings/` | ready | 725 | 56 | JNI | Class inheritance (2); Callback-shaped APIs (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/` | blocked | 263 | 56 | JNI | Concurrency (22); Class inheritance (4) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/` | ready | 304 | 55 | JNI | Null references (6); Integer width and overflow (5) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` | blocked | 33 | 54 | JNI | Callback-shaped APIs (2); Class inheritance (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/add/` | caution | 285 | 52 | JNI | Concurrency (18); Callback-shaped APIs (6) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/qrcode/` | blocked | 193 | 52 | JNI | Integer width and overflow (6); Concurrency (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/theme/` | blocked | 487 | 52 | JNI | Null references (6); Integer width and overflow (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/` | blocked | 582 | 50 | JNI | Division and modulo (3); Callback-shaped APIs (2) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/perhostsettings/` | caution | 284 | 49 | JNI | String offsets and slicing (2); Concurrency (6) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` | blocked | 490 | 47 | JNI | Null references (8); Dynamic typing (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/updater/` | blocked | 164 | 43 | JNI | Concurrency (9); Dynamic typing (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/` | blocked | 332 | 41 | JNI | Null references (10); Dynamic typing (8) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/item/` | blocked | 477 | 41 | JNI | Class inheritance (34); Concurrency (34) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/` | blocked | 586 | 40 | JNI | Null references (10); Class inheritance (7) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/theme/` | blocked | 560 | 39 | JNI | Concurrency (10); Callback-shaped APIs (5) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/` | blocked | 482 | 34 | JNI | Callback-shaped APIs (9); Null references (8) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` | caution | 378 | 34 | JNI | Null references (21); Reflection and prototypes (20) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` | blocked | 104 | 33 | JNI | Null references (5); Concurrency (3) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/checksum/` | blocked | 309 | 30 | JNI | Class inheritance (12); Callback-shaped APIs (8) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/` | blocked | 466 | 30 | JNI | Dynamic typing (19); Callback-shaped APIs (8) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/notification/` | blocked | 64 | 26 | JNI | Class inheritance (1); Null references (1) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/multiple/` | blocked | 457 | 26 | JNI | Concurrency (16); Null references (16) |
| 0 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/` | blocked | 87 | 23 | JNI | Class inheritance (4); Concurrency (3) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/` | blocked | 627 | 16 | JNI | String offsets and slicing (14); Concurrency (11) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/` | blocked | 908 | 14 | JNI | Callback-shaped APIs (23); Concurrency (20) |
| 0 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/` | blocked | 804 | 6 | JNI | String offsets and slicing (3); Callback-shaped APIs (13) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/` | blocked | 3113 | 0 | JNI | Division and modulo (36); Null references (32) |
| 0 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/` | blocked | 1985 | 0 | JNI | Null references (34); Class inheritance (26) |
| 2 | `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` | ready | 117 | 78 | JNI | Null references (3); Class inheritance (2) |
| 2 | `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` | ready | 124 | 77 | JNI | Null references (3); Class inheritance (2) |
| 2 | `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` | caution | 235 | 67 | JNI | Null references (8); Class inheritance (3) |

### 16. shared/auto-start (`shared/auto-start`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/auto-start/src/commonMain/kotlin/ir/amirab/util/startup/` | ready | 8 | 100 | JNI | - |
| 0 | `shared/auto-start/src/androidMain/kotlin/ir/amirab/util/startup/` | blocked | 35 | 85 | JNI | Class inheritance (1) |
| 0 | `shared/auto-start/src/desktopMain/kotlin/ir/amirab/util/startup/` | ready | 255 | 78 | JNI | Class inheritance (5); Null references (2) |

### 17. shared/compose-utils (`shared/compose-utils`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/contants/` | ready | 8 | 100 | JNI | - |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/modifiers/` | blocked | 35 | 85 | JNI | Null references (1) |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/layout/` | blocked | 49 | 83 | JNI | Class inheritance (3) |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/resources/` | blocked | 26 | 83 | JNI | - |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/` | blocked | 171 | 66 | JNI | Class inheritance (5); Null references (4) |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/action/` | blocked | 209 | 59 | JNI | Null references (18); Class inheritance (11) |
| 0 | `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/localizationmanager/` | blocked | 437 | 49 | JNI | Concurrency (7); Null references (6) |

### 18. shared/config (`shared/config`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/config/src/main/kotlin/ir/amirab/util/config/` | ready | 72 | 82 | JNI | Dynamic typing (9); Null references (6) |
| 1 | `shared/config/src/main/kotlin/ir/amirab/util/config/datastore/` | blocked | 186 | 53 | JNI | Class inheritance (4); Callback-shaped APIs (2) |

### 19. shared/resources (`shared/resources`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/` | caution | 42 | 87 | JNI | - |
| 0 | `shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/icons/` | blocked | 5633 | 0 | JNI | Null references (79) |

### 20. shared/resources/contracts (`shared/resources/contracts`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/resources/contracts/src/commonMain/kotlin/ir/amirab/resources/contracts/` | ready | 25 | 84 | JNI | String offsets and slicing (1); Class inheritance (2) |

### 21. shared/updater (`shared/updater`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/github/` | ready | 50 | 85 | JNI | Null references (1) |
| 0 | `shared/updater/src/androidMain/kotlin/` | ready | 50 | 84 | JNI | Dynamic typing (2); Class inheritance (2) |
| 0 | `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/updateapplier/` | ready | 106 | 79 | JNI | String offsets and slicing (1); Dynamic typing (3) |
| 0 | `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/updatechecker/` | ready | 122 | 66 | JNI | String offsets and slicing (3); Null references (4) |
| 0 | `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/` | caution | 149 | 63 | JNI | Null references (9); Class inheritance (7) |
| 0 | `shared/updater/src/desktopMain/kotlin/com/abdownloadmanager/updateapplier/` | ready | 256 | 51 | JNI | String offsets and slicing (1); Class inheritance (3) |

### 22. shared/utils (`shared/utils`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `shared/utils/src/desktopMain/kotlin/ir/amirab/util/` | ready | 13 | 100 | JNI | - |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/osfileutil/` | caution | 67 | 92 | JNI | Class inheritance (2); Null references (1) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/` | ready | 67 | 90 | JNI | String offsets and slicing (2); Null references (1) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/datasize/` | caution | 298 | 75 | JNI | Integer width and overflow (8); Class inheritance (4) |
| 0 | `shared/utils/src/androidMain/kotlin/ir/amirab/util/` | blocked | 19 | 69 | JNI | Class inheritance (1) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/logger/` | caution | 125 | 68 | JNI | Null references (4); Concurrency (3) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/coroutines/` | caution | 114 | 67 | JNI | Dynamic typing (15); Concurrency (11) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/flow/` | caution | 494 | 52 | JNI | Dynamic typing (15); Callback-shaped APIs (11) |
| 0 | `shared/utils/src/desktopMain/kotlin/ir/amirab/util/osfileutil/` | caution | 300 | 52 | JNI | Null references (14); Class inheritance (8) |
| 0 | `shared/utils/src/commonMain/kotlin/ir/amirab/util/` | caution | 596 | 2 | JNI | String offsets and slicing (3); Null references (23) |
| 0 | `shared/utils/src/androidMain/kotlin/ir/amirab/util/osfileutil/` | blocked | 81 | 0 | JNI | Reflection and prototypes (6); Callback-shaped APIs (2) |

## Maven dependencies

| Package | Units | Rust | Fit | Note |
| --- | ---: | --- | --- | --- |
| `androidx.compose` | 131 | - | none | UI and lifecycle: an application rewrite, not a port. |
| `android.content` | 26 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.os` | 19 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `androidx.activity` | 18 | - | none | UI and lifecycle: an application rewrite, not a port. |
| `androidx.core` | 10 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.app` | 9 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.net` | 6 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `androidx.datastore` | 6 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.util` | 4 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.webkit` | 3 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.widget` | 3 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `androidx.lifecycle` | 3 | - | none | UI and lifecycle: an application rewrite, not a port. |
| `android.media` | 2 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.provider` | 2 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `androidx.annotation` | 2 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.Manifest` | 1 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.annotation` | 1 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.graphics` | 1 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `android.view` | 1 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `androidx.camera` | 1 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `ir.amirab` | 209 | ? | unknown |  |
| `com.abdownloadmanager` | 178 | ? | unknown |  |
| `org.koin` | 48 | ? | unknown |  |
| `com.arkivanov` | 47 | ? | unknown |  |
| `io.github` | 29 | ? | unknown |  |
| `com.github` | 21 | ? | unknown |  |
| `okio.Path` | 20 | ? | unknown |  |
| `arrow.core` | 16 | ? | unknown |  |
| `okio.FileSystem` | 10 | ? | unknown |  |
| `arrow.optics` | 8 | ? | unknown |  |
| `org.gradle` | 8 | ? | unknown |  |
| `com.mikepenz` | 5 | ? | unknown |  |
| `okhttp3.internal` | 5 | ? | unknown |  |
| `dev.nucleusframework` | 4 | ? | unknown |  |
| `okhttp3.OkHttpClient` | 4 | ? | unknown |  |
| `sh.calvin` | 4 | ? | unknown |  |
| `co.touchlab` | 3 | ? | unknown |  |
| `okhttp3.HttpUrl` | 3 | ? | unknown |  |
| `okio.*` | 3 | ? | unknown |  |
| `okio.Buffer` | 3 | ? | unknown |  |
| `okio.Source` | 3 | ? | unknown |  |
| `com.google` | 2 | ? | unknown |  |
| `kotlinx.collections` | 2 | ? | unknown |  |
| `kotlinx.rpc` | 2 | ? | unknown |  |
| `okhttp3.Call` | 2 | ? | unknown |  |
| `okhttp3.Dispatcher` | 2 | ? | unknown |  |
| `okhttp3.Protocol` | 2 | ? | unknown |  |
| `okhttp3.Response` | 2 | ? | unknown |  |
| `okio.Sink` | 2 | ? | unknown |  |
| `okio.buffer` | 2 | ? | unknown |  |
| `okio.use` | 2 | ? | unknown |  |
| `org.freedesktop` | 2 | ? | unknown |  |
| `AndroidDirectLinkUpdateApplier` | 1 | ? | unknown |  |
| `com.jetbrains` | 1 | ? | unknown |  |
| `io.lindstrom` | 1 | ? | unknown |  |
| `okhttp3.*` | 1 | ? | unknown |  |
| `okhttp3.Callback` | 1 | ? | unknown |  |
| `okhttp3.Dns` | 1 | ? | unknown |  |
| `okhttp3.Request` | 1 | ? | unknown |  |
| `okhttp3.coroutines` | 1 | ? | unknown |  |
| `okhttp3.dnsoverhttps` | 1 | ? | unknown |  |
| `okio.FileHandle` | 1 | ? | unknown |  |
| `okio.ForwardingSink` | 1 | ? | unknown |  |
| `okio.ForwardingSource` | 1 | ? | unknown |  |
| `okio.IOException` | 1 | ? | unknown |  |
| `okio.sink` | 1 | ? | unknown |  |
| `org.eclipse` | 1 | ? | unknown |  |
| `org.intellij` | 1 | ? | unknown |  |
| `kotlinx.coroutines` | 129 | tokio, rayon | partial | suspend functions stay in Kotlin in phase 1 |
| `io.ktor` | 3 | axum, reqwest | partial |  |
| `kotlinx.serialization` | 56 | serde, serde_json | good | Kotlinx serialization maps to serde |
| `kotlinx.datetime` | 10 | jiff | good | Kotlin's own date and time API carries over; the `java.time` type behind it does not. |
| `org.slf4j` | 3 | tracing | good |  |

## Blocked

- `android/app/src/main/kotlin/com/abdownloadmanager/android/`: `android.app` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/di/`: `android.app` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/about/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/`: `android.util` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/initialsetup/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/permissions/`: `android.Manifest` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/perhostsettings/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/service/`: `android.app` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/storage/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/page/`: `androidx.compose` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/widget/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/util/`: `android.annotation` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/`: `android.content` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/util/compose/`: `androidx.activity` has no Rust equivalent.
- `android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/`: `android.content` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/`: `androidx.compose` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/`: `androidx.compose` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/`: `androidx.compose` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/`: `androidx.compose` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/`: `androidx.compose` has no Rust equivalent.
- `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/updater/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/`: `androidx.datastore` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/template/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/util/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/`: `androidx.compose` has no Rust equivalent.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/`: `androidx.compose` has no Rust equivalent.
- `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/`: `androidx.compose` has no Rust equivalent.
- `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/screen/`: `androidx.compose` has no Rust equivalent.
- `downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/`: `android.app` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/modifier/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/theme/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/`: `android.app` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/`: `android.os` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/keepawake/`: `android.content` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/notification/`: `android.app` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/systemusage/`: `android.app` has no Rust equivalent.
- `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/`: `androidx.activity` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/addToQueue/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/multiple/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/shared/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/checksum/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/queue/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/updater/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/`: `androidx.datastore` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/impl/`: `androidx.datastore` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/item/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/modifier/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/theme/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/`: `androidx.annotation` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/mvi/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/qrcode/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/theme/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/modifier/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/theme/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/util/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/native/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/styled/`: `androidx.compose` has no Rust equivalent.
- `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/`: `androidx.compose` has no Rust equivalent.
- `shared/auto-start/src/androidMain/kotlin/ir/amirab/util/startup/`: `android.content` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/`: `androidx.compose` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/action/`: `androidx.compose` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/layout/`: `androidx.compose` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/localizationmanager/`: `androidx.compose` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/modifiers/`: `androidx.compose` has no Rust equivalent.
- `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/resources/`: `androidx.compose` has no Rust equivalent.
- `shared/config/src/main/kotlin/ir/amirab/util/config/datastore/`: `androidx.datastore` has no Rust equivalent.
- `shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/icons/`: `androidx.compose` has no Rust equivalent.
- `shared/utils/src/androidMain/kotlin/ir/amirab/util/`: `android.content` has no Rust equivalent.
- `shared/utils/src/androidMain/kotlin/ir/amirab/util/osfileutil/`: `android.content` has no Rust equivalent.

## Risk guide

- **String offsets and slicing** (High): JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross the boundary must be converted, or non-ASCII text gets wrong ranges silently.
- **Dynamic typing** (Medium): Values whose shape is checked at runtime need an explicit Rust enum or `serde_json::Value`.
- **Exceptions** (Low): Thrown errors become `Result`. Errors crossing the bridge must map back to the same JS error shape.
- **Class inheritance** (Medium): Class hierarchies become traits plus composition; `instanceof` checks become enums.
- **Reflection and prototypes** (Medium): Proxy, Reflect, Symbol and prototype tricks need a redesign, not a translation.
- **Callback-shaped APIs** (Medium): Function-typed parameters and methods cross the bridge on every call. Keep boundaries coarse: pass data in, get data out.
- **Integer width and overflow** (Medium): Python ints never overflow; C, Go and Java ints wrap or are undefined on overflow. Pick i64, i128 or a bignum crate deliberately, and use wrapping_* where the source wraps.
- **Division and modulo** (Medium): Python `//` and `%` floor toward negative infinity; Rust `/` and `%` truncate: use `div_euclid`/`rem_euclid` or explicit floor math. C division by zero is undefined.
- **Hash iteration order** (High): Set and map iteration order comes from the source language's hash function. A port cannot reproduce it: sort before output, or make the order part of the contract.
- **Concurrency** (Medium): Threads, goroutines and locks need a design in Rust (channels, Send/Sync), not a line-by-line translation; keep concurrency in the caller where possible.
- **Null references** (Medium): Any reference may be null in the source: decide per parameter whether it is `Option` and what null means, instead of mapping every value to `Option`.

## Whole app

Shape: **server**.

| Entry point | Kind | Found from |
| --- | --- | --- |
| `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/App.kt` | cli | `fun main(` at desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/App.kt:17 |
| `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/CliApp.kt` | cli | `fun main(` at desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/CliApp.kt:13 |
| `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt` | cli | `fun main(` at desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt:36 |
| `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt` | server | `createServer(` at desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt:25 |
| `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/PartSplitSupport.kt` | cli | `fun main(` at downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/PartSplitSupport.kt:5 |
| `integration/server/src/main/kotlin/com/abdownloadmanager/integration/Integration.kt` | server | `createServer(` at integration/server/src/main/kotlin/com/abdownloadmanager/integration/Integration.kt:63 |
| `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/Notification.kt` | cli | `fun main(` at shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/Notification.kt:287 |

### Every dependency

| Dependency | Declared | Used by | Rust | Fit | Note |
| --- | --- | --- | --- | --- | --- |
| `androidx.activity` (also `androidx.compose`, `androidx.lifecycle`) | no | 131 | - | none | UI and lifecycle: an application rewrite, not a port. |
| `android.Manifest` (also `android.annotation`, `android.app`, `android.content`, `android.graphics`, `android.media`, `android.net`, `android.os`, `android.provider`, `android.util`, `android.view`, `android.webkit`, `android.widget`, `androidx.annotation`, `androidx.camera`, `androidx.core`, `androidx.datastore`) | no | 37 | - | none | The Android framework is the platform the port runs under, not a library it replaces. |
| `ir.amirab` | no | 209 | ? | unknown |  |
| `com.abdownloadmanager` | no | 178 | ? | unknown |  |
| `org.koin` | no | 48 | ? | unknown |  |
| `com.arkivanov` | no | 47 | ? | unknown |  |
| `io.github` | no | 29 | ? | unknown |  |
| `com.github` | no | 21 | ? | unknown |  |
| `okio.Path` | no | 20 | ? | unknown |  |
| `arrow.core` | no | 16 | ? | unknown |  |
| `okio.FileSystem` | no | 10 | ? | unknown |  |
| `arrow.optics` | no | 8 | ? | unknown |  |
| `org.gradle` | no | 8 | ? | unknown |  |
| `com.mikepenz` | no | 5 | ? | unknown |  |
| `okhttp3.internal` | no | 5 | ? | unknown |  |
| `dev.nucleusframework` | no | 4 | ? | unknown |  |
| `okhttp3.OkHttpClient` | no | 4 | ? | unknown |  |
| `sh.calvin` | no | 4 | ? | unknown |  |
| `co.touchlab` | no | 3 | ? | unknown |  |
| `okhttp3.HttpUrl` | no | 3 | ? | unknown |  |
| `okio.*` | no | 3 | ? | unknown |  |
| `okio.Buffer` | no | 3 | ? | unknown |  |
| `okio.Source` | no | 3 | ? | unknown |  |
| `com.google` | no | 2 | ? | unknown |  |
| `kotlinx.collections` | no | 2 | ? | unknown |  |
| `kotlinx.rpc` | no | 2 | ? | unknown |  |
| `okhttp3.Call` | no | 2 | ? | unknown |  |
| `okhttp3.Dispatcher` | no | 2 | ? | unknown |  |
| `okhttp3.Protocol` | no | 2 | ? | unknown |  |
| `okhttp3.Response` | no | 2 | ? | unknown |  |
| `okio.Sink` | no | 2 | ? | unknown |  |
| `okio.buffer` | no | 2 | ? | unknown |  |
| `okio.use` | no | 2 | ? | unknown |  |
| `org.freedesktop` | no | 2 | ? | unknown |  |
| `AndroidDirectLinkUpdateApplier` | no | 1 | ? | unknown |  |
| `com.jetbrains` | no | 1 | ? | unknown |  |
| `io.lindstrom` | no | 1 | ? | unknown |  |
| `okhttp3.*` | no | 1 | ? | unknown |  |
| `okhttp3.Callback` | no | 1 | ? | unknown |  |
| `okhttp3.Dns` | no | 1 | ? | unknown |  |
| `okhttp3.Request` | no | 1 | ? | unknown |  |
| `okhttp3.coroutines` | no | 1 | ? | unknown |  |
| `okhttp3.dnsoverhttps` | no | 1 | ? | unknown |  |
| `okio.FileHandle` | no | 1 | ? | unknown |  |
| `okio.ForwardingSink` | no | 1 | ? | unknown |  |
| `okio.ForwardingSource` | no | 1 | ? | unknown |  |
| `okio.IOException` | no | 1 | ? | unknown |  |
| `okio.sink` | no | 1 | ? | unknown |  |
| `org.eclipse` | no | 1 | ? | unknown |  |
| `org.intellij` | no | 1 | ? | unknown |  |
| `io.lindstrom:m3u8-parser` | yes | 0 | ? | unknown |  |
| `ir.amirab.plugin:common-android` | yes | 0 | ? | unknown |  |
| `ir.amirab.plugin:git-version-plugin` | yes | 0 | ? | unknown |  |
| `ir.amirab.plugin:installer-plugin` | yes | 0 | ? | unknown |  |
| `ir.amirab.util:platform` | yes | 0 | ? | unknown |  |
| `kotlinx.coroutines` | no | 129 | tokio, rayon | partial | suspend functions stay in Kotlin in phase 1 |
| `io.ktor` | no | 3 | axum, reqwest | partial |  |
| `kotlinx.serialization` | no | 56 | serde, serde_json | good | Kotlinx serialization maps to serde |
| `kotlinx.datetime` | no | 10 | jiff | good | Kotlin's own date and time API carries over; the `java.time` type behind it does not. |
| `org.slf4j` | no | 3 | tracing | good |  |

### Pieces, in port order

- wave 0: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/` (136 lines)
- wave 0: `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/page/` (270 lines)
- wave 0: `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/widget/` (683 lines)
- wave 0: `android/app/src/main/kotlin/com/abdownloadmanager/android/util/compose/` (45 lines)
- wave 0: `buildSrc/src/main/kotlin/Plugins.kt` (35 lines)
- wave 0: `buildSrc/src/main/kotlin/buildlogic/versioning/` (37 lines)
- wave 0: `compositeBuilds/plugins/common-android/src/main/kotlin/ir/amirab/plugin/common_android/task/` (229 lines)
- wave 0: `compositeBuilds/plugins/git-version-plugin/src/main/kotlin/ir/amirab/git_version/core/` (340 lines)
- wave 0: `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/tasks/macos/` (116 lines)
- wave 0: `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/utils/` (4 lines)
- wave 0: `compositeBuilds/shared/platform/src/commonMain/kotlin/ir/amirab/util/platform/` (137 lines)
- wave 0: `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/` (56 lines)
- wave 0: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/` (108 lines)
- wave 0: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/template/` (58 lines)
- wave 0: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/util/` (62 lines)
- wave 0: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/proxy/` (84 lines)
- wave 0: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/renderapi/` (104 lines)
- wave 0: `desktop/mac_utils/src/main/kotlin/ir/amirab/util/desktop/mac/event/` (39 lines)
- wave 0: `desktop/mac_utils/src/main/kotlin/ir/amirab/util/desktop/mac/native/` (4 lines)
- wave 0: `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/poweraction/` (81 lines)
- wave 0: `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/screen/` (66 lines)
- wave 0: `desktop/slf4j-impl/src/main/kotlin/ir/amirab/util/logger/slf4j/impl/` (102 lines)
- wave 0: `downloader/core/src/androidMain/kotlin/ir/amirab/downloader/utils/` (28 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/` (492 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/anntation/` (7 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/` (294 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/proxy/` (63 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/` (87 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/response/headers/` (137 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/db/` (397 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/destination/` (484 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/` (351 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/contexts/` (35 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/hls/` (1025 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/downloaditem/http/` (969 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/exception/` (113 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/` (749 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/queue/` (1132 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/utils/` (410 lines)
- wave 0: `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/utils/speedlimiter/` (162 lines)
- wave 0: `downloader/core/src/desktopMain/kotlin/ir/amirab/downloader/utils/` (67 lines)
- wave 0: `downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/` (596 lines)
- wave 0: `integration/server/src/main/kotlin/com/abdownloadmanager/integration/model/` (73 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/` (35 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` (6 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/theme/` (75 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/` (36 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` (86 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/` (87 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` (17 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` (33 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/notification/` (64 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` (104 lines)
- wave 0: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` (7 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/` (9 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/action/` (376 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/` (332 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/add/` (285 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/edit/` (315 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/` (304 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/add/` (226 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/hls/edit/` (293 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/` (248 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/add/` (253 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/edit/` (280 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderror/` (39 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pagemanager/` (92 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/` (184 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/addToQueue/` (75 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/multiple/` (457 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/shared/` (34 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/` (482 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/batchdownload/` (161 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/category/` (136 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/checksum/` (309 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/credits/translators/` (17 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/editdownload/` (95 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/enterurl/` (111 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/` (908 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/category/` (57 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/queue/` (123 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/perhostsettings/` (284 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/updater/` (164 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/repository/` (201 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/settings/` (725 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/` (466 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/` (263 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/impl/` (45 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/` (40 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/` (627 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/item/` (477 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` (14 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/theme/` (560 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/` (3113 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` (490 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/sort/` (64 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/updater/` (77 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/` (1985 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/appinfo/` (26 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/autoremove/` (156 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/` (586 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/di/` (3 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/dns/` (92 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/` (92 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/definederrors/` (421 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/faileddownloads/` (72 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` (8 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/extractors/` (4 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/extractors/linkextractor/` (159 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` (57 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/mvi/` (97 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/notification/` (53 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ondownloadcompletion/` (57 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/onqueuecompletion/` (63 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/perhostsettings/` (50 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/proxy/` (152 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/qrcode/` (193 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/schemakt/` (14 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` (54 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/` (582 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/icon/` (91 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/theme/` (487 lines)
- wave 0: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` (246 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/` (41 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/modifier/` (33 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/theme/` (125 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/util/` (7 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/` (41 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/` (145 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/native/` (94 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/` (804 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/styled/` (38 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/` (56 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/` (89 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/keepawake/` (80 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/notification/` (42 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/systemusage/` (378 lines)
- wave 0: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/` (6 lines)
- wave 0: `shared/auto-start/src/androidMain/kotlin/ir/amirab/util/startup/` (35 lines)
- wave 0: `shared/auto-start/src/commonMain/kotlin/ir/amirab/util/startup/` (8 lines)
- wave 0: `shared/auto-start/src/desktopMain/kotlin/ir/amirab/util/startup/` (255 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/` (171 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/action/` (209 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/contants/` (8 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/layout/` (49 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/localizationmanager/` (437 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/modifiers/` (35 lines)
- wave 0: `shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/resources/` (26 lines)
- wave 0: `shared/config/src/main/kotlin/ir/amirab/util/config/` (72 lines)
- wave 0: `shared/resources/contracts/src/commonMain/kotlin/ir/amirab/resources/contracts/` (25 lines)
- wave 0: `shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/` (42 lines)
- wave 0: `shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/icons/` (5633 lines)
- wave 0: `shared/updater/src/androidMain/kotlin/` (50 lines)
- wave 0: `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/` (149 lines)
- wave 0: `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/github/` (50 lines)
- wave 0: `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/updateapplier/` (106 lines)
- wave 0: `shared/updater/src/commonMain/kotlin/com/abdownloadmanager/updatechecker/` (122 lines)
- wave 0: `shared/updater/src/desktopMain/kotlin/com/abdownloadmanager/updateapplier/` (256 lines)
- wave 0: `shared/utils/src/androidMain/kotlin/ir/amirab/util/` (19 lines)
- wave 0: `shared/utils/src/androidMain/kotlin/ir/amirab/util/osfileutil/` (81 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/` (67 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/` (596 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/coroutines/` (114 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/datasize/` (298 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/flow/` (494 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/logger/` (125 lines)
- wave 0: `shared/utils/src/commonMain/kotlin/ir/amirab/util/osfileutil/` (67 lines)
- wave 0: `shared/utils/src/desktopMain/kotlin/ir/amirab/util/` (13 lines)
- wave 0: `shared/utils/src/desktopMain/kotlin/ir/amirab/util/osfileutil/` (300 lines)
- wave 1: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/about/` (406 lines)
- wave 1: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/initialsetup/` (150 lines)
- wave 1: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/permissions/` (725 lines)
- wave 1: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/perhostsettings/` (372 lines)
- wave 1: `compositeBuilds/plugins/git-version-plugin/src/main/kotlin/ir/amirab/git_version/` (13 lines)
- wave 1: `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/`, `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/extensiion/`, `compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/tasks/windows/`, tangled: one piece (303 lines)
- wave 1: `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/`, `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/activator/mac/`, `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/dock/mac/`, `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/linux/`, `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/mac/`, `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/windows/`, tangled: one piece (377 lines)
- wave 1: `integration/server/src/main/kotlin/com/abdownloadmanager/integration/` (209 lines)
- wave 1: `shared/config/src/main/kotlin/ir/amirab/util/config/datastore/` (186 lines)
- wave 2: `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/item/` (27 lines)
- wave 2: `buildSrc/src/main/kotlin/buildlogic/` (184 lines)
- wave 2: `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/` (248 lines)
- wave 2: `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/`, `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/`, `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/`, `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/`, `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/`, tangled: one piece (2088 lines)
- wave 2: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/repository/` (72 lines)
- wave 2: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/` (2237 lines)
- wave 2: `shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` (117 lines)
- wave 2: `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` (235 lines)
- wave 2: `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/storage/appsettings/` (124 lines)
- wave 3: `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/` (14 lines)
- wave 3: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/` (432 lines)
- wave 3: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/updater/` (348 lines)
- wave 3: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/` (38 lines)
- wave 4: `android/app/src/main/kotlin/com/abdownloadmanager/android/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/action/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/di/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/repository/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/service/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/storage/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/util/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/`, `android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/`, tangled: one piece (22884 lines)
- wave 4: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/`, `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/`, tangled: one piece (19533 lines)
- wave 5: `android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/` (79 lines)

### Decisions for the user

**D1** (shape): The app is a server, entered through desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/App.kt, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/CliApp.kt, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/NativeMessagingHost.kt, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/SingleInstanceServer.kt, downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/PartSplitSupport.kt, integration/server/src/main/kotlin/com/abdownloadmanager/integration/Integration.kt, shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/Notification.kt. Port every entry point, in that shape?

> Yes: an axum front door (`app front-door`) takes the server over first

**D2** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/): android/app/src/main/kotlin/com/abdownloadmanager/android/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D3** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/di/): android/app/src/main/kotlin/com/abdownloadmanager/android/di/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D4** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/about/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/about/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D5** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D6** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D7** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D8** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D9** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D10** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D11** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D12** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D13** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D14** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D15** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D16** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D17** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D18** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D19** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D20** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D21** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/ is blocked: `android.util` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D22** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D23** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D24** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D25** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D26** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D27** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/initialsetup/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/initialsetup/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D28** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/permissions/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/onboarding/permissions/ is blocked: `android.Manifest` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D29** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/perhostsettings/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/perhostsettings/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D30** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D31** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D32** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D33** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D34** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/): android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D35** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/): android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D36** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/service/): android/app/src/main/kotlin/com/abdownloadmanager/android/service/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D37** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/storage/): android/app/src/main/kotlin/com/abdownloadmanager/android/storage/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D38** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D39** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D40** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D41** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D42** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D43** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/page/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/page/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D44** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/ui/widget/): android/app/src/main/kotlin/com/abdownloadmanager/android/ui/widget/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D45** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/util/): android/app/src/main/kotlin/com/abdownloadmanager/android/util/ is blocked: `android.annotation` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D46** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/): android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D47** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/util/compose/): android/app/src/main/kotlin/com/abdownloadmanager/android/util/compose/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D48** (blocked android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/): android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D49** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D50** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D51** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D52** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D53** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D54** (blocked desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D55** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D56** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D57** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D58** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D59** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D60** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D61** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D62** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D63** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D64** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D65** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D66** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D67** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D68** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D69** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D70** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D71** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D72** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D73** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D74** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D75** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D76** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D77** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D78** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D79** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/updater/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/updater/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D80** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/ is blocked: `androidx.datastore` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D81** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D82** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D83** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D84** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D85** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D86** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/template/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/template/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D87** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/util/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/util/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D88** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D89** (blocked desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D90** (blocked desktop/shared/src/main/kotlin/ir/amirab/util/desktop/): desktop/shared/src/main/kotlin/ir/amirab/util/desktop/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D91** (blocked desktop/shared/src/main/kotlin/ir/amirab/util/desktop/screen/): desktop/shared/src/main/kotlin/ir/amirab/util/desktop/screen/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D92** (blocked downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/): downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D93** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D94** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/modifier/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/modifier/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D95** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/theme/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/theme/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D96** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D97** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D98** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D99** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/downloadlocation/ is blocked: `android.os` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D100** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/keepawake/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/keepawake/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D101** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/notification/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/notification/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D102** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/systemusage/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/systemusage/ is blocked: `android.app` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D103** (blocked shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/): shared/app/src/androidMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/ is blocked: `androidx.activity` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D104** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D105** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/addToQueue/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/addToQueue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D106** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/multiple/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/multiple/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D107** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/shared/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/shared/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D108** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D109** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/checksum/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/checksum/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D110** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D111** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/queue/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/queue/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D112** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/updater/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/updater/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D113** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D114** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/ is blocked: `androidx.datastore` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D115** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/impl/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/impl/ is blocked: `androidx.datastore` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D116** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D117** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D118** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/item/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/configurable/item/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D119** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/modifier/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/modifier/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D120** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/theme/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/theme/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D121** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/ is blocked: `androidx.annotation` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D122** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D123** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D124** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D125** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/downloaderror/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D126** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/mvi/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/mvi/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D127** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/qrcode/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/qrcode/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D128** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D129** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/theme/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/theme/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D130** (blocked shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/): shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D131** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/modifier/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/modifier/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D132** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/theme/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/theme/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D133** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/util/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/util/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D134** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D135** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/custom/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D136** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/native/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/menu/native/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D137** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D138** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/styled/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/ui/widget/table/customtable/styled/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D139** (blocked shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/): shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ui/widget/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D140** (blocked shared/auto-start/src/androidMain/kotlin/ir/amirab/util/startup/): shared/auto-start/src/androidMain/kotlin/ir/amirab/util/startup/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D141** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D142** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/action/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/action/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D143** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/layout/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/layout/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D144** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/localizationmanager/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/localizationmanager/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D145** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/modifiers/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/modifiers/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D146** (blocked shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/resources/): shared/compose-utils/src/commonMain/kotlin/ir/amirab/util/compose/resources/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D147** (blocked shared/config/src/main/kotlin/ir/amirab/util/config/datastore/): shared/config/src/main/kotlin/ir/amirab/util/config/datastore/ is blocked: `androidx.datastore` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D148** (blocked shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/icons/): shared/resources/src/commonMain/kotlin/com/abdownloadmanager/resources/icons/ is blocked: `androidx.compose` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D149** (blocked shared/utils/src/androidMain/kotlin/ir/amirab/util/): shared/utils/src/androidMain/kotlin/ir/amirab/util/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D150** (blocked shared/utils/src/androidMain/kotlin/ir/amirab/util/osfileutil/): shared/utils/src/androidMain/kotlin/ir/amirab/util/osfileutil/ is blocked: `android.content` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D151** (dependency androidx.activity): No crate does what androidx.activity does (used by 131 units; UI and lifecycle: an application rewrite, not a port). Reimplement what the app uses, or drop the feature?

> Reimplement only the part the app uses, proved by its recorded cases

**D152** (dependency android.Manifest): No crate does what android.Manifest does (used by 37 units; The Android framework is the platform the port runs under, not a library it replaces). Reimplement what the app uses, or drop the feature?

> Reimplement only the part the app uses, proved by its recorded cases

**D153** (dependency ir.amirab): Which crate replaces ir.amirab (used by 209 units)?

> Research it against primary sources into rust/research/ir.amirab.md before porting what uses it

**D154** (dependency com.abdownloadmanager): Which crate replaces com.abdownloadmanager (used by 178 units)?

> Research it against primary sources into rust/research/com.abdownloadmanager.md before porting what uses it

**D155** (dependency org.koin): Which crate replaces org.koin (used by 48 units)?

> Research it against primary sources into rust/research/org.koin.md before porting what uses it

**D156** (dependency com.arkivanov): Which crate replaces com.arkivanov (used by 47 units)?

> Research it against primary sources into rust/research/com.arkivanov.md before porting what uses it

**D157** (dependency io.github): Which crate replaces io.github (used by 29 units)?

> Research it against primary sources into rust/research/io.github.md before porting what uses it

**D158** (dependency com.github): Which crate replaces com.github (used by 21 units)?

> Research it against primary sources into rust/research/com.github.md before porting what uses it

**D159** (dependency okio.Path): Which crate replaces okio.Path (used by 20 units)?

> Research it against primary sources into rust/research/okio.Path.md before porting what uses it

**D160** (dependency arrow.core): Which crate replaces arrow.core (used by 16 units)?

> Research it against primary sources into rust/research/arrow.core.md before porting what uses it

**D161** (dependency okio.FileSystem): Which crate replaces okio.FileSystem (used by 10 units)?

> Research it against primary sources into rust/research/okio.FileSystem.md before porting what uses it

**D162** (dependency arrow.optics): Which crate replaces arrow.optics (used by 8 units)?

> Research it against primary sources into rust/research/arrow.optics.md before porting what uses it

**D163** (dependency org.gradle): Which crate replaces org.gradle (used by 8 units)?

> Research it against primary sources into rust/research/org.gradle.md before porting what uses it

**D164** (dependency com.mikepenz): Which crate replaces com.mikepenz (used by 5 units)?

> Research it against primary sources into rust/research/com.mikepenz.md before porting what uses it

**D165** (dependency okhttp3.internal): Which crate replaces okhttp3.internal (used by 5 units)?

> Research it against primary sources into rust/research/okhttp3.internal.md before porting what uses it

**D166** (dependency dev.nucleusframework): Which crate replaces dev.nucleusframework (used by 4 units)?

> Research it against primary sources into rust/research/dev.nucleusframework.md before porting what uses it

**D167** (dependency okhttp3.OkHttpClient): Which crate replaces okhttp3.OkHttpClient (used by 4 units)?

> Research it against primary sources into rust/research/okhttp3.OkHttpClient.md before porting what uses it

**D168** (dependency sh.calvin): Which crate replaces sh.calvin (used by 4 units)?

> Research it against primary sources into rust/research/sh.calvin.md before porting what uses it

**D169** (dependency co.touchlab): Which crate replaces co.touchlab (used by 3 units)?

> Research it against primary sources into rust/research/co.touchlab.md before porting what uses it

**D170** (dependency okhttp3.HttpUrl): Which crate replaces okhttp3.HttpUrl (used by 3 units)?

> Research it against primary sources into rust/research/okhttp3.HttpUrl.md before porting what uses it

**D171** (dependency okio.*): Which crate replaces okio.* (used by 3 units)?

> Research it against primary sources into rust/research/okio.*.md before porting what uses it

**D172** (dependency okio.Buffer): Which crate replaces okio.Buffer (used by 3 units)?

> Research it against primary sources into rust/research/okio.Buffer.md before porting what uses it

**D173** (dependency okio.Source): Which crate replaces okio.Source (used by 3 units)?

> Research it against primary sources into rust/research/okio.Source.md before porting what uses it

**D174** (dependency com.google): Which crate replaces com.google (used by 2 units)?

> Research it against primary sources into rust/research/com.google.md before porting what uses it

**D175** (dependency kotlinx.collections): Which crate replaces kotlinx.collections (used by 2 units)?

> Research it against primary sources into rust/research/kotlinx.collections.md before porting what uses it

**D176** (dependency kotlinx.rpc): Which crate replaces kotlinx.rpc (used by 2 units)?

> Research it against primary sources into rust/research/kotlinx.rpc.md before porting what uses it

**D177** (dependency okhttp3.Call): Which crate replaces okhttp3.Call (used by 2 units)?

> Research it against primary sources into rust/research/okhttp3.Call.md before porting what uses it

**D178** (dependency okhttp3.Dispatcher): Which crate replaces okhttp3.Dispatcher (used by 2 units)?

> Research it against primary sources into rust/research/okhttp3.Dispatcher.md before porting what uses it

**D179** (dependency okhttp3.Protocol): Which crate replaces okhttp3.Protocol (used by 2 units)?

> Research it against primary sources into rust/research/okhttp3.Protocol.md before porting what uses it

**D180** (dependency okhttp3.Response): Which crate replaces okhttp3.Response (used by 2 units)?

> Research it against primary sources into rust/research/okhttp3.Response.md before porting what uses it

**D181** (dependency okio.Sink): Which crate replaces okio.Sink (used by 2 units)?

> Research it against primary sources into rust/research/okio.Sink.md before porting what uses it

**D182** (dependency okio.buffer): Which crate replaces okio.buffer (used by 2 units)?

> Research it against primary sources into rust/research/okio.buffer.md before porting what uses it

**D183** (dependency okio.use): Which crate replaces okio.use (used by 2 units)?

> Research it against primary sources into rust/research/okio.use.md before porting what uses it

**D184** (dependency org.freedesktop): Which crate replaces org.freedesktop (used by 2 units)?

> Research it against primary sources into rust/research/org.freedesktop.md before porting what uses it

**D185** (dependency AndroidDirectLinkUpdateApplier): Which crate replaces AndroidDirectLinkUpdateApplier (used by 1 unit)?

> Research it against primary sources into rust/research/AndroidDirectLinkUpdateApplier.md before porting what uses it

**D186** (dependency com.jetbrains): Which crate replaces com.jetbrains (used by 1 unit)?

> Research it against primary sources into rust/research/com.jetbrains.md before porting what uses it

**D187** (dependency io.lindstrom): Which crate replaces io.lindstrom (used by 1 unit)?

> Research it against primary sources into rust/research/io.lindstrom.md before porting what uses it

**D188** (dependency okhttp3.*): Which crate replaces okhttp3.* (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.*.md before porting what uses it

**D189** (dependency okhttp3.Callback): Which crate replaces okhttp3.Callback (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.Callback.md before porting what uses it

**D190** (dependency okhttp3.Dns): Which crate replaces okhttp3.Dns (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.Dns.md before porting what uses it

**D191** (dependency okhttp3.Request): Which crate replaces okhttp3.Request (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.Request.md before porting what uses it

**D192** (dependency okhttp3.coroutines): Which crate replaces okhttp3.coroutines (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.coroutines.md before porting what uses it

**D193** (dependency okhttp3.dnsoverhttps): Which crate replaces okhttp3.dnsoverhttps (used by 1 unit)?

> Research it against primary sources into rust/research/okhttp3.dnsoverhttps.md before porting what uses it

**D194** (dependency okio.FileHandle): Which crate replaces okio.FileHandle (used by 1 unit)?

> Research it against primary sources into rust/research/okio.FileHandle.md before porting what uses it

**D195** (dependency okio.ForwardingSink): Which crate replaces okio.ForwardingSink (used by 1 unit)?

> Research it against primary sources into rust/research/okio.ForwardingSink.md before porting what uses it

**D196** (dependency okio.ForwardingSource): Which crate replaces okio.ForwardingSource (used by 1 unit)?

> Research it against primary sources into rust/research/okio.ForwardingSource.md before porting what uses it

**D197** (dependency okio.IOException): Which crate replaces okio.IOException (used by 1 unit)?

> Research it against primary sources into rust/research/okio.IOException.md before porting what uses it

**D198** (dependency okio.sink): Which crate replaces okio.sink (used by 1 unit)?

> Research it against primary sources into rust/research/okio.sink.md before porting what uses it

**D199** (dependency org.eclipse): Which crate replaces org.eclipse (used by 1 unit)?

> Research it against primary sources into rust/research/org.eclipse.md before porting what uses it

**D200** (dependency org.intellij): Which crate replaces org.intellij (used by 1 unit)?

> Research it against primary sources into rust/research/org.intellij.md before porting what uses it

**D201** (dependency io.lindstrom:m3u8-parser): Which crate replaces io.lindstrom:m3u8-parser (declared, no unit imports it)?

> Research it against primary sources into rust/research/io.lindstrom-m3u8-parser.md before porting what uses it

**D202** (dependency ir.amirab.plugin:common-android): Which crate replaces ir.amirab.plugin:common-android (declared, no unit imports it)?

> Research it against primary sources into rust/research/ir.amirab.plugin-common-android.md before porting what uses it

**D203** (dependency ir.amirab.plugin:git-version-plugin): Which crate replaces ir.amirab.plugin:git-version-plugin (declared, no unit imports it)?

> Research it against primary sources into rust/research/ir.amirab.plugin-git-version-plugin.md before porting what uses it

**D204** (dependency ir.amirab.plugin:installer-plugin): Which crate replaces ir.amirab.plugin:installer-plugin (declared, no unit imports it)?

> Research it against primary sources into rust/research/ir.amirab.plugin-installer-plugin.md before porting what uses it

**D205** (dependency ir.amirab.util:platform): Which crate replaces ir.amirab.util:platform (declared, no unit imports it)?

> Research it against primary sources into rust/research/ir.amirab.util-platform.md before porting what uses it

**D206** (dependency kotlinx.coroutines): kotlinx.coroutines maps to tokio, rayon only in part (used by 129 units; suspend functions stay in Kotlin in phase 1). Use it?

> Use tokio, rayon, and record cases for the behavior it does not cover

**D207** (dependency io.ktor): io.ktor maps to axum, reqwest only in part (used by 3 units). Use it?

> Use axum, reqwest, and record cases for the behavior it does not cover

**D208** (tangled units): compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/, compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/extensiion/, compositeBuilds/plugins/installer-plugin/src/main/kotlin/ir/amirab/installer/tasks/windows/ import each other (303 lines). Port them as one piece?

> Yes: one piece in wave 1, no glue between its parts

**D209** (tangled units): desktop/shared/src/main/kotlin/ir/amirab/util/desktop/, desktop/shared/src/main/kotlin/ir/amirab/util/desktop/activator/mac/, desktop/shared/src/main/kotlin/ir/amirab/util/desktop/dock/mac/, desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/linux/, desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/mac/, desktop/shared/src/main/kotlin/ir/amirab/util/desktop/utils/windows/ import each other (377 lines). Port them as one piece?

> Yes: one piece in wave 1, no glue between its parts

**D210** (tangled units): desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/, desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/, desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/linux/, desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/mac/, desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/titlebar/windows/ import each other (2088 lines). Port them as one piece?

> Yes: one piece in wave 2, no glue between its parts

**D211** (tangled units): android/app/src/main/kotlin/com/abdownloadmanager/android/, android/app/src/main/kotlin/com/abdownloadmanager/android/action/, android/app/src/main/kotlin/com/abdownloadmanager/android/di/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/multiple/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/shared/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/add/single/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/batchdownload/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/browser/bookmark/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/category/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/checksum/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/crashreport/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/thirdpartylibraries/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/credits/translators/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/directorypicker/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/downloaderror/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/editdownload/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/enterurl/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/queues/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/home/sections/sort/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/newqueue/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/qrcode/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/queue/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/settings/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/singledownload/, android/app/src/main/kotlin/com/abdownloadmanager/android/pages/updater/, android/app/src/main/kotlin/com/abdownloadmanager/android/receiver/, android/app/src/main/kotlin/com/abdownloadmanager/android/repository/, android/app/src/main/kotlin/com/abdownloadmanager/android/service/, android/app/src/main/kotlin/com/abdownloadmanager/android/storage/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/android/renderer/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/configurable/comon/renderer/, android/app/src/main/kotlin/com/abdownloadmanager/android/ui/menu/, android/app/src/main/kotlin/com/abdownloadmanager/android/util/, android/app/src/main/kotlin/com/abdownloadmanager/android/util/activity/, android/app/src/main/kotlin/com/abdownloadmanager/android/util/pagemanager/ import each other (22884 lines). Port them as one piece?

> Yes: one piece in wave 4, no glue between its parts

**D212** (tangled units): desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/actions/onevennts/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/hls/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/http/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/add/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/pause/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/remove/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/resume/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/download/show/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/exit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/gui/startifnotstarted/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/install/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/run/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/cli/nativemessaging/uninstall/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/di/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/stdio/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/about/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/multiple/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/shared/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/batchdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/checksum/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/confirmexit/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/credits/translators/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/downloaderror/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/editdownload/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/extenallibs/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/category/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/newQueue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/perhostsettings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/poweractionalert/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/storage/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/item/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/platform/renderer/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/error/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/widget/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/, desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/singleInstance/service/ import each other (19533 lines). Port them as one piece?

> Yes: one piece in wave 4, no glue between its parts


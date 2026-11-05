# Reelearn mobile

One Flutter app: the employee minute (sign-in, Today, the swipe stack, Saved,
Progress, Settings) and the manager screens (Team readiness, Record a reel),
role-gated by the `role` claim of the signed-in user.

## Run

```sh
cd mobile
flutter pub get                 # also runs gen-l10n (lib/app/l10n/gen/)
flutter run -d ios              # or: flutter run -d android
```

### API base URL and fake mode

| define | default | notes |
|---|---|---|
| `API_BASE` | `http://localhost:8080/v1` | On an Android emulator `localhost` is rewritten to `10.0.2.2` automatically. |
| `API` | `http` | `fake` swaps in the in-memory `FakeApi` (demo data, no server). |

```sh
# against the real API on a device on the same Wi-Fi
flutter run --dart-define=API_BASE=http://192.168.1.20:8080/v1

# no server at all: demo tenant "alwadi", Sara +966500000010 (learner, ar),
# Khalid +966500000002 (manager, en), OTP code 1234
flutter run --dart-define=API=fake
```

Deep link: `reelearn://t/alwadi?inv=<token>` opens sign-in with the company
already resolved (iOS `CFBundleURLSchemes` and the Android intent filter are
registered for the `reelearn` scheme).

## Tests, lint, format

```sh
flutter analyze
dart format --set-exit-if-changed .
flutter test
```

All tests run against `FakeApi`; no network, no fonts, no video platform.

## Layout

```
lib/app          router (go_router), theme (brand tokens), l10n (ARB en/ar/ur), shell
lib/core         api (ReelearnApi, HttpApi, FakeApi, models, Dio interceptors),
                 auth (token store, session), offline (queue, connectivity), errors
lib/features/<auth|today|player|saved|progress|settings|team|record>/{data,domain,presentation}
```

* No business rules in widgets: gating, segments and tallies live in
  `features/player/domain/stack_model.dart`; the recorder state machine in
  `features/record/domain/record_state.dart`.
* No HTTP outside `lib/core/api`.
* Models are hand-written immutable classes with `fromJson`/`toJson`
  (no build_runner step), one per schema in `openapi/openapi.yaml`.
* Telemetry: `POST /me/watch` on arrival and at 90 %, `POST /me/answers` on
  pick, each with a uuid v4 `Idempotency-Key` persisted in the offline queue
  (`shared_preferences`) and replayed once on reconnect.
* Tokens live in `flutter_secure_storage`; a 401 refreshes once and retries.

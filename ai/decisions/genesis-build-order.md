# Почему `ValidatorSet` имеет меньший индекс палеты, чем `Session`

**Дата:** 2026-07-19. **Статус:** действует. **Область:** `runtime/src/lib.rs` (блок
`#[frame_support::runtime] mod runtime`), `pallets/validator-set`.

## Короткое правило

В рантайме `pallet-validator-set` (индекс 7) объявлен **раньше** `pallet-session` (индекс 8).
Этот порядок обязателен. Не меняй его местами и не вставляй новую палету так, чтобы `Session`
получил индекс меньше, чем `ValidatorSet`.

## Почему это важно

FRAME строит генезис-состояние, вызывая genesis-сборку каждой палеты **строго по возрастанию
индекса палеты**. Это не порядок объявления строк в файле, а именно порядок индексов: макрос
`#[frame_support::runtime]` перед сборкой сортирует палеты вызовом `pallets.sort_by_key(|p| p.index)`
(см. `frame/support/procedural/src/runtime/expand/mod.rs`, поле `legacy_ordering = false`), и уже
этот отсортированный список идёт в экспандер `RuntimeGenesisConfig::build`.

`pallet-session` в своей genesis-сборке спрашивает начальный набор валидаторов у своего
`SessionManager` — а в нашем рантайме `SessionManager` — это `pallet-validator-set`:

```rust
// pallet-session, genesis build (substrate/frame/session/src/lib.rs)
let initial_validators_0 = T::SessionManager::new_session_genesis(0).unwrap_or_else(|| {
    // ... "No initial validator provided by SessionManager, use session config keys ..."
    self.keys.iter().map(|x| x.1.clone()).collect()
});
// ... затем этот набор через on_genesis_session наполняет авторитетов Aura и GRANDPA.
```

Наш `SessionManager::new_session` (и унаследованный `new_session_genesis`) возвращает
`Some(Validators::<T>::get())` — то есть содержимое хранилища `Validators` нашей палеты. Это
хранилище засевается genesis-сборкой самой `pallet-validator-set` (`Validators::put(initial_validators)`).

Отсюда жёсткая зависимость: **наша палета должна засеять `Validators` до того, как `session` его
прочитает.** Это выполняется тогда и только тогда, когда индекс `ValidatorSet` меньше индекса
`Session`.

## Что ломается при неправильном порядке (проверено вживую)

Если `Session` собирается раньше (как было в первой редакции — Session(7), ValidatorSet(9)), то в
момент опроса `Validators::get()` ещё пуст, `new_session` возвращает `Some(пустой вектор)` — не
`None`, поэтому запасной путь session (взять валидаторов из своих `keys`) НЕ включается. Набор
нулевой сессии оказывается пустым, авторитеты Aura и GRANDPA не наполняются, и узел **паникует при
старте**:

```
Thread 'main' panicked at 'genesis authorities is non-empty; all weights are non-zero; qed.',
substrate/client/consensus/grandpa/src/aux_schema.rs:395
```

Паника происходит в `new_partial` узла (GRANDPA `block_import` → `load_persistent`), которая читает
`GrandpaApi::grandpa_authorities()` на генезис-состоянии и требует непустой набор. Ошибка не
зависит от пресета: воспроизводилась и на `--dev` (JSON-генезис в `chain_spec.rs`), и на
типизированном `pilier_testnet` (`genesis_config_presets.rs`) — то есть это не проблема JSON-ключей,
а именно порядка сборки.

## Рассмотренная альтернатива (отвергнута)

Можно было оставить Session(7)/ValidatorSet(9) и переопределить в нашей палете
`new_session_genesis(_) -> None`, чтобы session взял начальный набор из своих собственных
genesis-`keys`. Это работает и даёт на генезисе идентичный результат (те же три валидатора),
потому что `session.keys` и `validator_set.initial_validators` перечисляют одни и те же аккаунты.
Но это «запасной» путь session (менеджер отказывается называть набор), при котором формальным
источником истины на генезисе становится `session.keys`, а не палета набора. Для тестнета разница
косметическая, но mainnet собирается на базе этого кода, и правильнее иметь единый источник истины —
палету набора — уже на генезисе. Поэтому выбрана перенумерация (индекс `ValidatorSet` < `Session`),
а `new_session_genesis` оставлен со стандартной реализацией (наследует `new_session`).

## Следствие для будущих палет

`pallet-collective` (Фаза 4 плана `runtime-mutable-validator-set`) получает индекс 9. У него нет
genesis-зависимости от порядка относительно session/validator-set (его genesis лишь засевает список
членов), поэтому индекс 9 безопасен. Любую новую палету с genesis-логикой, которая читается другой
палетой на генезисе, размещай с учётом этого же правила «поставщик — раньше потребителя по индексу».

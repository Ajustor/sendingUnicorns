//! Composition root: the only place that knows every layer (R-ARCH-05).

use std::sync::Arc;

use anyhow::Context as _;
use application::collections::{
    CreateCollection, DeleteCollection, ExportCollection, ImportCollection, LoadWorkspace,
    RenameCollection, SaveCollections,
};
use application::history::{LoadHistory, RecordHistory};
use application::ports::{
    CollectionArchive, CollectionRepository, GlobalsRepository, HistoryRepository, HttpClient,
    SettingsRepository,
};
use application::runner::RunCollection;
use application::sending::SendRequest;
use application::settings::{ChangeSettings, LoadSettings, ManageCookies};
use infrastructure::{
    JsonCollectionArchive, JsonCollectionRepository, JsonGlobalsRepository, JsonHistoryRepository,
    JsonSettingsRepository, UreqHttpClient,
};
use updater::{UpdateMemory, Updater};

use crate::ui::{ExitSlot, Preview, SendingApp, Updates, UseCases};

pub fn build(cc: &eframe::CreationContext<'_>, exit_slot: ExitSlot) -> anyhow::Result<SendingApp> {
    crate::ui::theme::install(&cc.egui_ctx);

    let data_dir =
        infrastructure::data_dir().context("no data directory to keep collections in")?;
    let collections_dir = data_dir.join("collections");
    if let Some(legacy) = infrastructure::legacy_collections_dir() {
        match infrastructure::migrate_legacy(&legacy, &collections_dir) {
            Ok(0) => {}
            Ok(n) => log::info!("{n} collection(s) brought over from {}", legacy.display()),
            Err(e) => log::warn!(
                "old collections in {} not brought over: {e}",
                legacy.display()
            ),
        }
    }

    let repository: Arc<dyn CollectionRepository> =
        Arc::new(JsonCollectionRepository::new(collections_dir));
    let archive: Arc<dyn CollectionArchive> = Arc::new(JsonCollectionArchive);
    let settings_repository: Arc<dyn SettingsRepository> =
        Arc::new(JsonSettingsRepository::new(data_dir.join("settings.json")));
    let globals: Arc<dyn GlobalsRepository> =
        Arc::new(JsonGlobalsRepository::new(data_dir.join("globals.json")));
    let history_repository: Arc<dyn HistoryRepository> =
        Arc::new(JsonHistoryRepository::new(data_dir.join("history.json")));
    let client: Arc<dyn HttpClient> = Arc::new(UreqHttpClient::default());

    let (mut workspace, problems) = LoadWorkspace::new(repository.clone())
        .execute()
        .context("could not read the collections")?;
    let mut warnings: Vec<String> = problems.iter().map(ToString::to_string).collect();
    match globals.load() {
        Ok(rows) => workspace.set_globals(rows),
        Err(e) => warnings.push(e.to_string()),
    }
    let (history, history_problem) = LoadHistory::new(history_repository.clone()).execute();
    warnings.extend(history_problem.map(|e| format!("{e} (history not loaded)")));
    let (settings, settings_problem) = LoadSettings::new(settings_repository.clone()).execute();
    warnings.extend(settings_problem.map(|e| format!("{e} (default settings used)")));

    let send = SendRequest::new(client.clone());
    let use_cases = UseCases {
        create: CreateCollection::new(repository.clone()),
        save: SaveCollections::new(repository.clone(), globals),
        rename: RenameCollection::new(repository.clone()),
        delete: DeleteCollection::new(repository.clone()),
        import: ImportCollection::new(archive.clone(), repository),
        export: ExportCollection::new(archive),
        run: RunCollection::new(send.clone()),
        send,
        change_settings: ChangeSettings::new(settings_repository),
        cookies: ManageCookies::new(client),
        history: RecordHistory::new(history_repository),
    };

    let ctx = cc.egui_ctx.clone();
    let updates = Updates::new(
        Updater::new(&crate::APP, move || ctx.request_repaint()),
        UpdateMemory::new(&data_dir),
    );
    Ok(SendingApp::new(
        &cc.egui_ctx,
        workspace,
        history,
        settings,
        use_cases,
        updates,
        exit_slot,
        Preview::new(cc),
        warnings,
    ))
}

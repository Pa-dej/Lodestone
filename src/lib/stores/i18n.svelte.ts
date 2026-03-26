export type UiLanguage = "ru" | "en";

const LANGUAGE_STORAGE_KEY = "lodestone-language";

const messages = {
  ru: {
    nav_servers: "Серверы",
    nav_console: "Консоль",
    nav_settings: "Настройки",
    brand_subtitle: "Лаунчер Minecraft-серверов",
    page_servers: "Серверы",
    page_console: "Консоль",
    page_settings: "Настройки",
    theme_dark: "Тёмная тема",
    theme_light: "Светлая тема",
    theme_system: "Системная тема",
    language_toggle: "Сменить язык интерфейса",
    add_server: "Добавить сервер",
    status_running: "Запущен",
    status_stopped: "Остановлен",
    action_start: "Запуск",
    action_stop: "Стоп",
    action_restart: "Рестарт",
    action_restarting: "Перезапуск...",
    action_console: "Консоль",
    action_clear: "Очистить",
    action_close: "Закрыть",
    action_save: "Сохранить",
    action_saving: "Сохранение...",
    action_cancel: "Отмена",
    action_delete: "Удалить",
    players_count: "игроков",
    delete_server_title: "Удалить сервер",
    open_in_explorer: "Открыть в проводнике",
    server_edit_profile: "Редактировать профиль сервера",
    error_title: "Ошибка",
    loading: "Загрузка...",
    save_success_title: "Сохранено",
    save_success_settings: "Настройки сохранены",
    delete_confirm_title: "Удаление сервера",
    delete_confirm_text: "Вы уверены, что хотите удалить сервер \"{name}\"? Это действие нельзя отменить.",
    modal_new_server: "Новый сервер",
    modal_next: "Далее",
    modal_back: "Назад",
    modal_create: "Создать",
    modal_wait: "Подождите...",
    modal_loading_versions: "Загрузка...",
    modal_no_versions: "Нет версий",
    field_server_name: "Имя сервера",
    field_version: "Версия",
    field_release_only: "Только релизы",
    field_port: "Порт",
    field_ram_mb: "RAM (MB)",
    field_jvm_args: "JVM args",
    field_motd: "MOTD",
    field_gamemode: "Режим игры",
    field_difficulty: "Сложность",
    field_view_distance: "Дальность прорисовки",
    field_simulation_distance: "Дальность симуляции",
    field_spawn_protection: "Защита спавна",
    field_max_players: "Макс. игроков",
    field_online_mode: "Лицензионный режим",
    field_online_mode_desc: "Проверка лицензии Mojang",
    field_pvp: "PVP",
    field_pvp_desc: "Разрешить урон между игроками",
    unit_chunks: "чанков",
    modal_select_version_error: "Выберите версию сервера",
    modal_create_error: "Не удалось создать сервер",
    modal_done_title: "Готово!",
    modal_done_subtitle: "Сервер создан успешно",
    modal_download_title: "Загрузка",
    modal_failed_title: "Ошибка",
    waterfall_deprecated_title: "Waterfall устарел",
    waterfall_deprecated_text: "Проект deprecated. Рекомендуется использовать Velocity.",
    settings_java: "Java",
    settings_application: "Приложение",
    settings_java_path: "Путь к Java",
    settings_max_ram: "Макс. RAM (MB)",
    settings_extra_jvm: "Доп. флаги JVM",
    settings_minimize_to_tray: "Сворачивать в трей",
    settings_minimize_to_tray_desc: "При закрытии окна приложение остаётся в системном трее.",
    settings_autostart_servers: "Автозапуск серверов",
    settings_autostart_servers_desc: "Запускать активные серверы сразу после старта приложения.",
    settings_kill_server_processes_on_exit: "Завершать процессы серверов при выходе",
    settings_kill_server_processes_on_exit_desc:
      "При полном закрытии Lodestone принудительно завершать оставшиеся процессы серверов.",
    settings_server_profile: "Профиль сервера",
    settings_server_properties: "server.properties",
    settings_server_properties_desc: "Быстрые параметры и полный список ключей для выбранного сервера.",
    settings_no_server_selected: "Выберите сервер, чтобы редактировать его свойства.",
    settings_reload_props: "Перезагрузить",
    settings_save_props: "Сохранить server.properties",
    settings_props_saved: "Свойства сервера сохранены",
    settings_common_group: "Быстрые настройки",
    settings_advanced_group: "Все параметры",
    settings_key: "Ключ",
    settings_value: "Значение",
    settings_add_property: "Добавить параметр",
    settings_remove_property: "Удалить",
    settings_whitelist: "Белый список",
    settings_allow_flight: "Разрешить полёт",
    settings_command_blocks: "Командные блоки",
    console_no_tabs: "Нет открытых вкладок.",
    console_tabs_button: "Вкладки",
    console_opened: "открыта",
    console_tab_search_placeholder: "Поиск сервера...",
    console_tab_search_empty: "Серверы не найдены",
    console_open_server: "Откройте сервер",
    console_empty_output: "Пока нет вывода консоли.",
    console_empty_hint: "Откройте вкладку сервера для просмотра консоли.",
    console_command_placeholder: "Введите команду...",
    console_available_commands: "Доступные команды",
    console_hint: "Tab: автодополнение • Двойной Tab: показать все варианты • ↑/↓: навигация • Enter: выбрать • Esc: закрыть",
  },
  en: {
    nav_servers: "Servers",
    nav_console: "Console",
    nav_settings: "Settings",
    brand_subtitle: "Minecraft Server Launcher",
    page_servers: "Servers",
    page_console: "Console",
    page_settings: "Settings",
    theme_dark: "Dark theme",
    theme_light: "Light theme",
    theme_system: "System theme",
    language_toggle: "Switch interface language",
    add_server: "Add server",
    status_running: "Running",
    status_stopped: "Stopped",
    action_start: "Start",
    action_stop: "Stop",
    action_restart: "Restart",
    action_restarting: "Restarting...",
    action_console: "Console",
    action_clear: "Clear",
    action_close: "Close",
    action_save: "Save",
    action_saving: "Saving...",
    action_cancel: "Cancel",
    action_delete: "Delete",
    players_count: "players",
    delete_server_title: "Delete server",
    open_in_explorer: "Open in Explorer",
    server_edit_profile: "Edit server profile",
    error_title: "Error",
    loading: "Loading...",
    save_success_title: "Saved",
    save_success_settings: "Settings saved",
    delete_confirm_title: "Delete server",
    delete_confirm_text: "Are you sure you want to delete server \"{name}\"? This action cannot be undone.",
    modal_new_server: "New server",
    modal_next: "Next",
    modal_back: "Back",
    modal_create: "Create",
    modal_wait: "Please wait...",
    modal_loading_versions: "Loading...",
    modal_no_versions: "No versions",
    field_server_name: "Server name",
    field_version: "Version",
    field_release_only: "Releases only",
    field_port: "Port",
    field_ram_mb: "RAM (MB)",
    field_jvm_args: "JVM args",
    field_motd: "MOTD",
    field_gamemode: "Gamemode",
    field_difficulty: "Difficulty",
    field_view_distance: "View distance",
    field_simulation_distance: "Simulation distance",
    field_spawn_protection: "Spawn protection",
    field_max_players: "Max players",
    field_online_mode: "Online mode",
    field_online_mode_desc: "Verify Mojang licenses",
    field_pvp: "PVP",
    field_pvp_desc: "Allow damage between players",
    unit_chunks: "chunks",
    modal_select_version_error: "Select a server version",
    modal_create_error: "Failed to create server",
    modal_done_title: "Done!",
    modal_done_subtitle: "Server created successfully",
    modal_download_title: "Downloading",
    modal_failed_title: "Error",
    waterfall_deprecated_title: "Waterfall is deprecated",
    waterfall_deprecated_text: "This project is deprecated. Velocity is recommended instead.",
    settings_java: "Java",
    settings_application: "Application",
    settings_java_path: "Java executable path",
    settings_max_ram: "Max RAM (MB)",
    settings_extra_jvm: "Extra JVM flags",
    settings_minimize_to_tray: "Minimize to tray",
    settings_minimize_to_tray_desc: "When closing window, app stays in the system tray.",
    settings_autostart_servers: "Autostart servers",
    settings_autostart_servers_desc: "Start active servers automatically on app startup.",
    settings_kill_server_processes_on_exit: "Kill server processes on exit",
    settings_kill_server_processes_on_exit_desc:
      "Force-stop remaining server processes when Lodestone fully exits.",
    settings_server_profile: "Server profile",
    settings_server_properties: "server.properties",
    settings_server_properties_desc: "Quick controls and full key/value list for the selected server.",
    settings_no_server_selected: "Select a server to edit its properties.",
    settings_reload_props: "Reload",
    settings_save_props: "Save server.properties",
    settings_props_saved: "Server properties saved",
    settings_common_group: "Quick settings",
    settings_advanced_group: "All properties",
    settings_key: "Key",
    settings_value: "Value",
    settings_add_property: "Add property",
    settings_remove_property: "Remove",
    settings_whitelist: "Whitelist",
    settings_allow_flight: "Allow flight",
    settings_command_blocks: "Command blocks",
    console_no_tabs: "No open tabs.",
    console_tabs_button: "Tabs",
    console_opened: "opened",
    console_tab_search_placeholder: "Search server...",
    console_tab_search_empty: "No servers found",
    console_open_server: "Open a server",
    console_empty_output: "No console output yet.",
    console_empty_hint: "Open a server tab to view the console.",
    console_command_placeholder: "Enter command...",
    console_available_commands: "Available commands",
    console_hint: "Tab: autocomplete • Double Tab: show all • ↑/↓: navigate • Enter: select • Esc: close",
  },
} as const;

export type TranslationKey = keyof (typeof messages)["ru"];

interface I18nState {
  language: UiLanguage;
  initialized: boolean;
}

export const i18nState = $state<I18nState>({
  language: "ru",
  initialized: false,
});

function resolveInitialLanguage(): UiLanguage {
  if (typeof window === "undefined") {
    return "ru";
  }

  const saved = localStorage.getItem(LANGUAGE_STORAGE_KEY);
  if (saved === "ru" || saved === "en") {
    return saved;
  }

  const browserLang = navigator.language.toLowerCase();
  return browserLang.startsWith("ru") ? "ru" : "en";
}

export function initializeLanguage(): void {
  if (i18nState.initialized) {
    return;
  }
  i18nState.language = resolveInitialLanguage();
  i18nState.initialized = true;
}

export function setLanguage(language: UiLanguage): void {
  i18nState.language = language;
  if (typeof window !== "undefined") {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  }
}

export function toggleLanguage(): void {
  setLanguage(i18nState.language === "ru" ? "en" : "ru");
}

export function t(key: TranslationKey): string {
  return messages[i18nState.language][key];
}

export function format(message: string, replacements: Record<string, string>): string {
  return Object.entries(replacements).reduce(
    (result, [key, value]) => result.replaceAll(`{${key}}`, value),
    message,
  );
}

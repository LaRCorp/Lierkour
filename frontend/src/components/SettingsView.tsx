import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { EcosystemAppStatus, ExtensionInfo, SyncServerStatus } from '../types';
import {
  Settings,
  Moon,
  Sun,
  Key,
  CheckCircle,
  Puzzle,
  Copy,
  Check,
  RefreshCw,
  Eye,
  EyeOff,
  AlertTriangle,
  Smartphone,
  Power,
  Radio,
  Wifi,
  Globe,
} from 'lucide-react';

interface SettingsViewProps {
  currentTheme: string;
  onThemeChange: (theme: string) => void;
  apiKey: string;
  onSaveApiKey: (key: string) => Promise<void>;
  extensionInfo: ExtensionInfo | null;
  onRegenerateExtensionToken: () => Promise<void>;
  onToggleExtensionServer: (enabled: boolean, port?: number) => Promise<void>;
  syncStatus: SyncServerStatus | null;
  onToggleSyncServer: (enabled: boolean, port?: number) => Promise<void>;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  currentTheme,
  onThemeChange,
  apiKey: initialKey,
  onSaveApiKey,
  extensionInfo,
  onRegenerateExtensionToken,
  onToggleExtensionServer,
  syncStatus,
  onToggleSyncServer,
}) => {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState(initialKey);
  const [saved, setSaved] = useState(false);
  const [copiedToken, setCopiedToken] = useState(false);
  const [copiedSyncUrl, setCopiedSyncUrl] = useState(false);
  const [showToken, setShowToken] = useState(false);
  const [regenerating, setRegenerating] = useState(false);

  const [extPort, setExtPort] = useState(
    extensionInfo?.status?.port ? String(extensionInfo.status.port) : '8765'
  );
  const [syncPort, setSyncPort] = useState(
    syncStatus?.port ? String(syncStatus.port) : '8767'
  );

  const [isTogglingExt, setIsTogglingExt] = useState(false);
  const [isTogglingSync, setIsTogglingSync] = useState(false);
  const [ecosystemApps, setEcosystemApps] = useState<EcosystemAppStatus[]>([]);
  const [loadingEco, setLoadingEco] = useState(false);

  const refreshEcosystem = async () => {
    setLoadingEco(true);
    try {
      const list = await invoke<EcosystemAppStatus[]>('probe_lier_ecosystem');
      setEcosystemApps(list);
    } catch {
      // ignore in browser preview
    } finally {
      setLoadingEco(false);
    }
  };

  useEffect(() => {
    refreshEcosystem();
  }, []);

  const isExtensionActive = Boolean(extensionInfo?.status.active);
  const isSyncActive = Boolean(syncStatus?.active);

  const syncIps = syncStatus?.local_ips && syncStatus.local_ips.length > 0
    ? syncStatus.local_ips
    : ['127.0.0.1'];
  const primarySyncIp = syncIps[0];
  const activeSyncPort = syncStatus?.port || 8767;
  const syncConnectionUrl = `http://${primarySyncIp}:${activeSyncPort}`;

  const handleSave = async () => {
    await onSaveApiKey(apiKey.trim());
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  const handleCopyToken = async () => {
    if (!extensionInfo?.token) return;
    try {
      await navigator.clipboard.writeText(extensionInfo.token);
      setCopiedToken(true);
      setTimeout(() => setCopiedToken(false), 2000);
    } catch (err) {
      console.error('Failed to copy token:', err);
    }
  };

  const handleCopySyncUrl = async () => {
    try {
      await navigator.clipboard.writeText(syncConnectionUrl);
      setCopiedSyncUrl(true);
      setTimeout(() => setCopiedSyncUrl(false), 2000);
    } catch (err) {
      console.error('Failed to copy sync URL:', err);
    }
  };

  const handleRegenerate = async () => {
    setRegenerating(true);
    try {
      await onRegenerateExtensionToken();
      setCopiedToken(false);
    } finally {
      setRegenerating(false);
    }
  };

  const handleToggleExt = async () => {
    setIsTogglingExt(true);
    try {
      const port = parseInt(extPort, 10);
      const validPort = isNaN(port) || port <= 0 ? 8765 : port;
      await onToggleExtensionServer(!isExtensionActive, validPort);
    } finally {
      setIsTogglingExt(false);
    }
  };

  const handleToggleSync = async () => {
    setIsTogglingSync(true);
    try {
      const port = parseInt(syncPort, 10);
      const validPort = isNaN(port) || port <= 0 ? 8767 : port;
      await onToggleSyncServer(!isSyncActive, validPort);
    } finally {
      setIsTogglingSync(false);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl animate-in fade-in duration-200 pb-10">
      <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
        <Settings className="w-5 h-5 text-indigo-500" />
        <span>{t('settings.title')}</span>
      </h2>

      {/* Theme Setting */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-3 shadow-sm">
        <div>
          <h3 className="text-sm font-bold text-slate-900 dark:text-slate-200">
            {t('settings.theme_title')}
          </h3>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            {t('settings.theme_desc')}
          </p>
        </div>

        <div className="flex items-center gap-3 pt-1">
          <button
            onClick={() => onThemeChange('dark')}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all cursor-pointer ${
              currentTheme === 'dark'
                ? 'border-indigo-500 bg-indigo-500/20 text-indigo-400 shadow-md shadow-indigo-500/10'
                : 'border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Moon className="w-4 h-4 text-indigo-400" />
            <span>{t('settings.theme_dark')}</span>
          </button>

          <button
            onClick={() => onThemeChange('light')}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all cursor-pointer ${
              currentTheme === 'light'
                ? 'border-indigo-500 bg-indigo-50 text-indigo-700 shadow-md shadow-indigo-500/10'
                : 'border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Sun className="w-4 h-4 text-amber-500" />
            <span>{t('settings.theme_light')}</span>
          </button>
        </div>
      </div>

      {/* SECTION: EXTERNAL ACCESS & NETWORKING */}
      <div className="pt-2">
        <h3 className="text-sm font-bold text-slate-900 dark:text-slate-200 uppercase tracking-wider mb-3">
          {t('settings.network_title')}
        </h3>

        <div className="space-y-4">
          {/* Card 1: Lierkour Mobile Synchronization */}
          <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-4 shadow-sm">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <div className="flex items-center gap-2 text-cyan-600 dark:text-cyan-400 font-bold text-sm">
                  <Smartphone className="w-4 h-4" />
                  <span>{t('settings.sync_mobile_title')}</span>
                </div>
                <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 leading-relaxed max-w-xl">
                  {t('settings.sync_mobile_desc')}
                </p>
              </div>

              {/* Status pill & Controls */}
              <div className="flex items-center gap-2.5">
                <span
                  className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-semibold border ${
                    isSyncActive
                      ? 'border-cyan-500/30 bg-cyan-500/10 text-cyan-500'
                      : 'border-slate-400/30 bg-slate-400/10 text-slate-400'
                  }`}
                >
                  {isSyncActive ? (
                    <CheckCircle className="w-3.5 h-3.5" />
                  ) : (
                    <AlertTriangle className="w-3.5 h-3.5" />
                  )}
                  <span>
                    {isSyncActive
                      ? `Порт ${activeSyncPort} открыт`
                      : 'Порт закрыт'}
                  </span>
                </span>

                <div className="flex items-center gap-1.5 bg-slate-100 dark:bg-slate-950 px-2.5 py-1 rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">Порт:</span>
                  <input
                    type="text"
                    value={syncPort}
                    disabled={isSyncActive || isTogglingSync}
                    onChange={(e) => setSyncPort(e.target.value.replace(/\D/g, '').slice(0, 5))}
                    className="w-12 bg-transparent text-xs font-mono font-bold text-slate-800 dark:text-slate-200 focus:outline-none disabled:opacity-60"
                  />
                </div>

                <button
                  type="button"
                  onClick={handleToggleSync}
                  disabled={isTogglingSync}
                  className={`inline-flex items-center gap-1.5 rounded-xl px-3.5 py-1.5 text-xs font-bold transition-all shadow-sm cursor-pointer disabled:opacity-50 ${
                    isSyncActive
                      ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-rose-600/20'
                      : 'bg-cyan-600 hover:bg-cyan-500 text-white shadow-cyan-600/20'
                  }`}
                >
                  <Power className="w-3.5 h-3.5" />
                  <span>{isSyncActive ? t('settings.sync_close_port') : t('settings.sync_open_port')}</span>
                </button>
              </div>
            </div>

            {/* If Active: Show Connection URL */}
            {isSyncActive ? (
              <div className="space-y-3 pt-1">
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                    {t('settings.sync_address_label')}
                  </label>
                  <div className="flex flex-wrap sm:flex-nowrap gap-2">
                    <div className="relative flex-1 min-w-[220px] bg-cyan-500/5 dark:bg-cyan-950/20 border border-cyan-500/30 rounded-xl px-3 py-2 flex items-center">
                      <Radio className="w-4 h-4 text-cyan-500 mr-2 shrink-0" />
                      <span className="font-mono text-xs sm:text-sm font-bold text-cyan-600 dark:text-cyan-400 select-all truncate">
                        {syncConnectionUrl}
                      </span>
                    </div>

                    <button
                      type="button"
                      onClick={handleCopySyncUrl}
                      className="inline-flex items-center gap-1.5 rounded-xl bg-cyan-600 px-4 py-2 text-xs font-semibold text-white shadow-lg shadow-cyan-600/20 hover:bg-cyan-500 transition-all cursor-pointer"
                    >
                      {copiedSyncUrl ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                      <span>{copiedSyncUrl ? t('settings.sync_url_copied') : t('settings.sync_copy_url')}</span>
                    </button>
                  </div>
                </div>

                <div className="flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
                  <Wifi className="w-3.5 h-3.5 text-cyan-500 shrink-0" />
                  <span>
                    Введите этот адрес в мобильном приложении Lierkour Mobile на телефоне (раздел «Синхронизация»).
                  </span>
                </div>
              </div>
            ) : (
              <p className="text-[11px] text-slate-500 dark:text-slate-400">
                Входящие сетевые подключения для мобильной синхронизации отключены. Порт {activeSyncPort} свободен.
              </p>
            )}
          </div>

          {/* Card 2: LierEX Browser Extension */}
          <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-4 shadow-sm">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <div className="flex items-center gap-2 text-indigo-600 dark:text-indigo-400 font-bold text-sm">
                  <Puzzle className="w-4 h-4" />
                  <span>{t('settings.lierex_title')}</span>
                </div>
                <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 leading-relaxed max-w-xl">
                  {t('settings.lierex_desc')}
                </p>
              </div>

              {/* Status pill & Controls */}
              <div className="flex items-center gap-2.5">
                <span
                  className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-semibold border ${
                    isExtensionActive
                      ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                      : 'border-rose-500/30 bg-rose-500/10 text-rose-600 dark:text-rose-400'
                  }`}
                >
                  {isExtensionActive ? (
                    <CheckCircle className="w-3.5 h-3.5" />
                  ) : (
                    <AlertTriangle className="w-3.5 h-3.5" />
                  )}
                  <span>
                    {isExtensionActive
                      ? `Порт ${extensionInfo?.status.port || 8765} открыт`
                      : 'Порт закрыт'}
                  </span>
                </span>

                <div className="flex items-center gap-1.5 bg-slate-100 dark:bg-slate-950 px-2.5 py-1 rounded-xl border border-slate-200 dark:border-slate-800">
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">Порт:</span>
                  <input
                    type="text"
                    value={extPort}
                    disabled={isExtensionActive || isTogglingExt}
                    onChange={(e) => setExtPort(e.target.value.replace(/\D/g, '').slice(0, 5))}
                    className="w-12 bg-transparent text-xs font-mono font-bold text-slate-800 dark:text-slate-200 focus:outline-none disabled:opacity-60"
                  />
                </div>

                <button
                  type="button"
                  onClick={handleToggleExt}
                  disabled={isTogglingExt}
                  className={`inline-flex items-center gap-1.5 rounded-xl px-3.5 py-1.5 text-xs font-bold transition-all shadow-sm cursor-pointer disabled:opacity-50 ${
                    isExtensionActive
                      ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-rose-600/20'
                      : 'bg-indigo-600 hover:bg-indigo-500 text-white shadow-indigo-600/20'
                  }`}
                >
                  <Power className="w-3.5 h-3.5" />
                  <span>{isExtensionActive ? t('settings.lierex_close_port') : t('settings.lierex_open_port')}</span>
                </button>
              </div>
            </div>

            {/* Token Section */}
            <div>
              <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">
                {t('settings.lierex_token_label')}
              </label>
              <div className="flex flex-wrap gap-2">
                <div className="relative flex-1 min-w-[200px]">
                  <input
                    type={showToken ? 'text' : 'password'}
                    readOnly
                    value={extensionInfo?.token || ''}
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 pl-3.5 pr-9 py-2 text-xs text-slate-900 dark:text-slate-100 font-mono focus:border-indigo-500 focus:outline-none"
                  />
                  <button
                    type="button"
                    onClick={() => setShowToken((v) => !v)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer"
                    aria-label="Toggle token visibility"
                  >
                    {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                  </button>
                </div>

                <button
                  type="button"
                  onClick={handleCopyToken}
                  disabled={!extensionInfo?.token}
                  className="inline-flex items-center gap-1.5 rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 disabled:opacity-50 transition-all cursor-pointer"
                >
                  {copiedToken ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>{copiedToken ? t('settings.lierex_copied') : t('settings.lierex_copy')}</span>
                </button>

                <button
                  type="button"
                  onClick={handleRegenerate}
                  disabled={regenerating}
                  className="inline-flex items-center gap-1.5 rounded-xl border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800/80 px-3.5 py-2 text-xs font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700 disabled:opacity-50 transition-all cursor-pointer"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${regenerating ? 'animate-spin' : ''}`} />
                  <span>{t('settings.lierex_regenerate')}</span>
                </button>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-1.5">
                {t('settings.lierex_hint')}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* OMDb API Key */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-4 shadow-sm">
        <div>
          <div className="flex items-center gap-2 text-indigo-600 dark:text-indigo-400 font-bold text-sm">
            <Key className="w-4 h-4" />
            <span>{t('settings.omdb_title')}</span>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 leading-relaxed">
            {t('settings.omdb_desc')}
          </p>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">
            {t('settings.omdb_key_label')}
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t('settings.omdb_key_placeholder')}
              className="flex-1 rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono text-xs"
            />
            <button
              onClick={handleSave}
              className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 transition-all cursor-pointer"
            >
              {t('settings.save')}
            </button>
          </div>
        </div>

        {saved && (
          <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400">
            <CheckCircle className="w-3.5 h-3.5" />
            <span>Настройки успешно сохранены</span>
          </div>
        )}
      </div>

      {/* LIER Ecosystem Status Card */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-3 shadow-sm">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 font-bold text-sm text-teal-600 dark:text-teal-400">
            <Globe className="w-4 h-4" />
            <span>Экосистема LIER · Lierkour v2.3 (GPLv3)</span>
          </div>
          <button
            type="button"
            onClick={refreshEcosystem}
            disabled={loadingEco}
            className="inline-flex items-center gap-1.5 rounded-xl border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800 px-3 py-1.5 text-xs font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700 cursor-pointer"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingEco ? 'animate-spin' : ''}`} />
            <span>Обновить</span>
          </button>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
          {ecosystemApps.map((app) => (
            <div
              key={app.id}
              className="flex items-center justify-between rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/70 px-3 py-2"
            >
              <div>
                <div className="font-semibold text-slate-900 dark:text-slate-100">{app.name}</div>
                <div className="text-[11px] text-slate-500">127.0.0.1:{app.port}</div>
              </div>
              <span
                className={`rounded-md px-2 py-0.5 font-mono text-[11px] ${
                  app.online
                    ? 'bg-teal-500/15 text-teal-500'
                    : 'bg-slate-200 dark:bg-slate-800 text-slate-500'
                }`}
              >
                {app.online ? `ONLINE (${app.detail})` : 'offline'}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

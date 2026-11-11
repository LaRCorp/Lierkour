import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ExtensionInfo } from '../types';
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
} from 'lucide-react';

interface SettingsViewProps {
  currentTheme: string;
  onThemeChange: (theme: string) => void;
  apiKey: string;
  onSaveApiKey: (key: string) => Promise<void>;
  extensionInfo: ExtensionInfo | null;
  onRegenerateExtensionToken: () => Promise<void>;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  currentTheme,
  onThemeChange,
  apiKey: initialKey,
  onSaveApiKey,
  extensionInfo,
  onRegenerateExtensionToken,
}) => {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState(initialKey);
  const [saved, setSaved] = useState(false);
  const [copiedToken, setCopiedToken] = useState(false);
  const [showToken, setShowToken] = useState(false);
  const [regenerating, setRegenerating] = useState(false);

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

  const handleRegenerate = async () => {
    setRegenerating(true);
    try {
      await onRegenerateExtensionToken();
      setCopiedToken(false);
    } finally {
      setRegenerating(false);
    }
  };

  const isExtensionActive = Boolean(extensionInfo?.status.active);

  return (
    <div className="space-y-6 max-w-2xl animate-in fade-in duration-200">
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
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all ${
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
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all ${
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

      {/* LierEX Browser Extension Connection */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-4 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <div className="flex items-center gap-2 text-indigo-600 dark:text-indigo-400 font-bold text-sm">
              <Puzzle className="w-4 h-4" />
              <span>{t('settings.lierex_title')}</span>
            </div>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 leading-relaxed">
              {t('settings.lierex_desc')}
            </p>
          </div>

          {extensionInfo && (
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
                  ? `${t('settings.lierex_active')} (${extensionInfo.status.address})`
                  : `${t('settings.lierex_inactive')}${
                      extensionInfo.status.error ? `: ${extensionInfo.status.error}` : ''
                    }`}
              </span>
            </span>
          )}
        </div>

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
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                aria-label="Toggle token visibility"
              >
                {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>

            <button
              type="button"
              onClick={handleCopyToken}
              disabled={!extensionInfo?.token}
              className="inline-flex items-center gap-1.5 rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 disabled:opacity-50 transition-all"
            >
              {copiedToken ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
              <span>{copiedToken ? t('settings.lierex_copied') : t('settings.lierex_copy')}</span>
            </button>

            <button
              type="button"
              onClick={handleRegenerate}
              disabled={regenerating}
              className="inline-flex items-center gap-1.5 rounded-xl border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800/80 px-3.5 py-2 text-xs font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700 disabled:opacity-50 transition-all"
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
              className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 transition-all"
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
    </div>
  );
};

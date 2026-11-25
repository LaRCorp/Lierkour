import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SyncServerStatus } from '../types';
import {
  Smartphone,
  Wifi,
  Copy,
  Check,
  RefreshCw,
  Power,
  ShieldCheck,
  ArrowLeftRight,
  AlertTriangle,
  CheckCircle,
  HelpCircle,
  Radio,
} from 'lucide-react';

interface SyncViewProps {
  syncStatus: SyncServerStatus | null;
  onToggleSyncServer: (enabled: boolean, port?: number) => Promise<void>;
  onRefreshSyncStatus: () => Promise<void>;
}

export const SyncView: React.FC<SyncViewProps> = ({
  syncStatus,
  onToggleSyncServer,
  onRefreshSyncStatus,
}) => {
  const { t } = useTranslation();
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [selectedIpIndex, setSelectedIpIndex] = useState(0);
  const [customPort, setCustomPort] = useState(syncStatus?.port ? String(syncStatus.port) : '8767');
  const [isToggling, setIsToggling] = useState(false);

  const isActive = Boolean(syncStatus?.active);
  const localIps = syncStatus?.local_ips && syncStatus.local_ips.length > 0
    ? syncStatus.local_ips
    : ['127.0.0.1'];

  const selectedIp = localIps[selectedIpIndex] || localIps[0];
  const activePort = syncStatus?.port || 8767;
  const connectionUrl = `http://${selectedIp}:${activePort}`;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(connectionUrl);
      setCopiedUrl(true);
      setTimeout(() => setCopiedUrl(false), 2500);
    } catch (err) {
      console.error('Failed to copy URL:', err);
    }
  };

  const handleToggle = async () => {
    setIsToggling(true);
    try {
      const portNum = parseInt(customPort, 10);
      const validPort = isNaN(portNum) || portNum <= 0 ? 8767 : portNum;
      await onToggleSyncServer(!isActive, validPort);
    } finally {
      setIsToggling(false);
    }
  };

  const handlePortChange = (val: string) => {
    // Only numbers allowed
    const sanitized = val.replace(/\D/g, '').slice(0, 5);
    setCustomPort(sanitized);
  };

  return (
    <div className="space-y-6 max-w-4xl mx-auto animate-in fade-in duration-200 pb-12">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-extrabold text-slate-900 dark:text-slate-100 flex items-center gap-2.5">
            <Smartphone className="w-7 h-7 text-cyan-500" />
            <span>{t('settings.sync_mobile_title')}</span>
          </h2>
          <p className="text-xs sm:text-sm text-slate-500 dark:text-slate-400 mt-1 max-w-2xl leading-relaxed">
            {t('settings.sync_mobile_desc')}
          </p>
        </div>

        <button
          onClick={onRefreshSyncStatus}
          title="Обновить статус"
          className="p-2.5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 text-slate-600 dark:text-slate-400 hover:text-cyan-500 dark:hover:text-cyan-400 transition-all cursor-pointer shadow-sm"
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      </div>

      {/* Main Status & Connection Card */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md space-y-6 shadow-sm">
        {/* Status banner and Port controls */}
        <div className="flex flex-wrap items-center justify-between gap-4 pb-4 border-b border-slate-100 dark:border-slate-800/80">
          <div className="flex items-center gap-3">
            <div
              className={`w-3.5 h-3.5 rounded-full ${
                isActive ? 'bg-cyan-500 animate-pulse shadow-lg shadow-cyan-500/50' : 'bg-slate-400 dark:bg-slate-600'
              }`}
            />
            <div>
              <div className="flex items-center gap-2">
                <span className="text-sm font-bold text-slate-900 dark:text-slate-100">
                  {isActive ? t('settings.sync_active') : t('settings.sync_inactive')}
                </span>
                <span
                  className={`text-[10px] font-bold px-2 py-0.5 rounded-full border ${
                    isActive
                      ? 'border-cyan-500/30 bg-cyan-500/10 text-cyan-500'
                      : 'border-slate-400/30 bg-slate-400/10 text-slate-400'
                  }`}
                >
                  Порт {activePort}
                </span>
              </div>
              <span className="text-xs text-slate-500 dark:text-slate-400">
                {isActive
                  ? `Слушает на 0.0.0.0:${activePort} (доступен устройствам в локальной сети)`
                  : 'Сетевой порт закрыт. Входящие подключения отключены.'}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-2.5">
            <div className="flex items-center gap-1.5 bg-slate-100 dark:bg-slate-950 px-3 py-1.5 rounded-xl border border-slate-200 dark:border-slate-800">
              <span className="text-xs text-slate-500 dark:text-slate-400 font-medium">Порт:</span>
              <input
                type="text"
                value={customPort}
                disabled={isActive || isToggling}
                onChange={(e) => handlePortChange(e.target.value)}
                className="w-14 bg-transparent text-xs font-mono font-bold text-slate-800 dark:text-slate-200 focus:outline-none disabled:opacity-60"
              />
            </div>

            <button
              onClick={handleToggle}
              disabled={isToggling}
              className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-bold transition-all shadow-md cursor-pointer disabled:opacity-50 ${
                isActive
                  ? 'bg-rose-600 text-white hover:bg-rose-500 shadow-rose-600/20'
                  : 'bg-cyan-600 text-white hover:bg-cyan-500 shadow-cyan-600/20'
              }`}
            >
              <Power className={`w-3.5 h-3.5 ${isToggling ? 'animate-spin' : ''}`} />
              <span>{isActive ? t('settings.sync_close_port') : t('settings.sync_open_port')}</span>
            </button>
          </div>
        </div>

        {/* If Active: Show Connection URL and Network Interface Selector */}
        {isActive ? (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-2">
                {t('settings.sync_address_label')}
              </label>

              <div className="flex flex-wrap sm:flex-nowrap items-center gap-2 bg-gradient-to-r from-cyan-500/5 via-slate-50 to-slate-50 dark:from-cyan-950/20 dark:via-slate-950 dark:to-slate-950 border border-cyan-500/30 rounded-2xl p-2.5 sm:p-3">
                <Radio className="w-5 h-5 text-cyan-500 shrink-0 ml-1 hidden sm:block" />
                <div className="flex-1 min-w-0 px-1 font-mono text-sm sm:text-base font-bold text-cyan-600 dark:text-cyan-400 select-all truncate">
                  {connectionUrl}
                </div>

                <button
                  type="button"
                  onClick={handleCopy}
                  className="inline-flex items-center gap-2 rounded-xl bg-cyan-600 px-5 py-2.5 text-xs font-bold text-white shadow-lg shadow-cyan-600/25 hover:bg-cyan-500 transition-all cursor-pointer shrink-0"
                >
                  {copiedUrl ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                  <span>{copiedUrl ? t('settings.sync_url_copied') : t('settings.sync_copy_url')}</span>
                </button>
              </div>
            </div>

            {/* If machine has multiple IPs (e.g. Wi-Fi, Ethernet, VPN) */}
            {localIps.length > 1 && (
              <div>
                <span className="text-xs font-medium text-slate-600 dark:text-slate-400 mb-2 block">
                  {t('settings.sync_detected_ips')}
                </span>
                <div className="flex flex-wrap gap-2">
                  {localIps.map((ip, idx) => (
                    <button
                      key={ip}
                      onClick={() => setSelectedIpIndex(idx)}
                      className={`px-3 py-1.5 rounded-xl text-xs font-mono transition-all border cursor-pointer ${
                        selectedIpIndex === idx
                          ? 'border-cyan-500 bg-cyan-500/15 text-cyan-500 font-bold shadow-sm'
                          : 'border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-600 dark:text-slate-400 hover:border-slate-300'
                      }`}
                    >
                      {ip}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="flex items-center gap-3 p-4 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-700 dark:text-amber-400 text-xs leading-relaxed">
            <AlertTriangle className="w-5 h-5 shrink-0" />
            <span>
              Порт синхронизации закрыт. Чтобы подключить мобильное устройство, нажмите кнопку{' '}
              <strong>«Открыть порт синхронизации»</strong> выше.
            </span>
          </div>
        )}
      </div>

      {/* Step by step Instruction Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 space-y-2 shadow-sm">
          <div className="w-8 h-8 rounded-xl bg-cyan-500/10 text-cyan-500 flex items-center justify-center font-bold text-sm">
            1
          </div>
          <h4 className="text-sm font-bold text-slate-900 dark:text-slate-200 flex items-center gap-1.5">
            <Wifi className="w-4 h-4 text-cyan-500" />
            <span>Общий Wi-Fi</span>
          </h4>
          <p className="text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
            {t('settings.sync_step_1')}
          </p>
        </div>

        <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 space-y-2 shadow-sm">
          <div className="w-8 h-8 rounded-xl bg-cyan-500/10 text-cyan-500 flex items-center justify-center font-bold text-sm">
            2
          </div>
          <h4 className="text-sm font-bold text-slate-900 dark:text-slate-200 flex items-center gap-1.5">
            <Smartphone className="w-4 h-4 text-cyan-500" />
            <span>Lierkour Mobile</span>
          </h4>
          <p className="text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
            {t('settings.sync_step_2')}
          </p>
        </div>

        <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 space-y-2 shadow-sm">
          <div className="w-8 h-8 rounded-xl bg-cyan-500/10 text-cyan-500 flex items-center justify-center font-bold text-sm">
            3
          </div>
          <h4 className="text-sm font-bold text-slate-900 dark:text-slate-200 flex items-center gap-1.5">
            <ArrowLeftRight className="w-4 h-4 text-cyan-500" />
            <span>Обмен данными</span>
          </h4>
          <p className="text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
            Вставьте скопированный адрес в телефон и нажмите «Синхронизировать сейчас».
          </p>
        </div>
      </div>

      {/* Feature Highlights & Firewall info */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 space-y-4 shadow-sm">
        <h4 className="text-sm font-bold text-slate-900 dark:text-slate-200 flex items-center gap-2">
          <ShieldCheck className="w-4 h-4 text-cyan-500" />
          <span>Преимущества локальной синхронизации Lierkour</span>
        </h4>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs text-slate-600 dark:text-slate-400">
          <div className="flex items-start gap-2 p-3 rounded-2xl bg-slate-50 dark:bg-slate-950 border border-slate-200/60 dark:border-slate-800/60">
            <CheckCircle className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
            <div>
              <strong className="text-slate-800 dark:text-slate-200">Без сторонних облаков:</strong> все данные
              передаются напрямую между вашим компьютером и телефоном по зашифрованному протоколу Wi-Fi.
            </div>
          </div>

          <div className="flex items-start gap-2 p-3 rounded-2xl bg-slate-50 dark:bg-slate-950 border border-slate-200/60 dark:border-slate-800/60">
            <CheckCircle className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
            <div>
              <strong className="text-slate-800 dark:text-slate-200">Бесконфликтное слияние (LWW):</strong> любые
              изменения, оценки или заметки на телефоне аккуратно объединяются с базой на ПК.
            </div>
          </div>

          <div className="flex items-start gap-2 p-3 rounded-2xl bg-slate-50 dark:bg-slate-950 border border-slate-200/60 dark:border-slate-800/60">
            <CheckCircle className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
            <div>
              <strong className="text-slate-800 dark:text-slate-200">Экономия памяти телефона:</strong> ПК автоматически
              создаёт сжатые превью обложек (до 400px), чтобы телефон не забивал хранилище.
            </div>
          </div>

          <div className="flex items-start gap-2 p-3 rounded-2xl bg-slate-50 dark:bg-slate-950 border border-slate-200/60 dark:border-slate-800/60">
            <CheckCircle className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
            <div>
              <strong className="text-slate-800 dark:text-slate-200">Полная безопасность:</strong> вы можете закрыть
              порт в любой момент кнопкой «Закрыть порт».
            </div>
          </div>
        </div>

        <div className="pt-2 flex items-start gap-2 text-[11px] text-slate-500 dark:text-slate-400">
          <HelpCircle className="w-4 h-4 text-slate-400 shrink-0 mt-0.5" />
          <span>
            {t('settings.sync_firewall_note', { port: activePort })}
          </span>
        </div>
      </div>
    </div>
  );
};


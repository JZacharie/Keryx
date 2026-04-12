"use client";

import React, { useState } from "react";
import {
  Settings,
  Globe,
  CheckCircle2,
  RotateCcw,
  Info,
} from "lucide-react";
import { motion } from "framer-motion";

const DEFAULT_API_URL = "https://ingestor.p.zacharie.org";

export default function SettingsPage() {
  const [apiUrl, setApiUrl] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("keryx_api_url") || DEFAULT_API_URL;
    }
    return DEFAULT_API_URL;
  });
  const [autoRefresh, setAutoRefresh] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("keryx_auto_refresh") !== "false";
    }
    return true;
  });
  const [refreshInterval, setRefreshInterval] = useState(() => {
    if (typeof window !== "undefined") {
      return parseInt(localStorage.getItem("keryx_refresh_interval") || "3", 10);
    }
    return 3;
  });
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    if (typeof window !== "undefined") {
      localStorage.setItem("keryx_api_url", apiUrl);
      localStorage.setItem("keryx_auto_refresh", String(autoRefresh));
      localStorage.setItem("keryx_refresh_interval", String(refreshInterval));
    }
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  const handleReset = () => {
    setApiUrl(DEFAULT_API_URL);
    setAutoRefresh(true);
    setRefreshInterval(3);
    if (typeof window !== "undefined") {
      localStorage.removeItem("keryx_api_url");
      localStorage.removeItem("keryx_auto_refresh");
      localStorage.removeItem("keryx_refresh_interval");
    }
  };

  return (
    <div className="space-y-8 pb-10">
      <div>
        <h2 className="text-4xl font-extrabold tracking-tight">Paramètres</h2>
        <p className="text-slate-400 font-medium mt-1">
          Configuration de l&apos;application
        </p>
      </div>

      {/* Connection settings */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        className="glass rounded-3xl p-8 border border-white/10 max-w-2xl space-y-6"
      >
        <div className="flex items-center gap-3 mb-2">
          <Globe className="w-5 h-5 text-[#8A2BE2]" />
          <h3 className="text-lg font-bold">Connexion API</h3>
        </div>

        <div>
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
            URL du backend Ingestor
          </label>
          <input
            type="url"
            value={apiUrl}
            onChange={(e) => setApiUrl(e.target.value)}
            className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl px-4 py-3.5 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all font-mono text-sm"
          />
          <p className="text-xs text-slate-600 mt-1.5">
            Défaut : {DEFAULT_API_URL}
          </p>
        </div>
      </motion.div>

      {/* Auto-refresh settings */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.05 }}
        className="glass rounded-3xl p-8 border border-white/10 max-w-2xl space-y-6"
      >
        <div className="flex items-center gap-3 mb-2">
          <Settings className="w-5 h-5 text-[#8A2BE2]" />
          <h3 className="text-lg font-bold">Comportement</h3>
        </div>

        {/* Auto refresh toggle */}
        <div className="flex items-center justify-between">
          <div>
            <p className="font-medium text-sm">Rafraîchissement automatique</p>
            <p className="text-xs text-slate-500 mt-0.5">
              Met à jour l&apos;état des jobs en cours automatiquement
            </p>
          </div>
          <button
            onClick={() => setAutoRefresh((v) => !v)}
            className={`w-12 h-6 rounded-full transition-all relative ${
              autoRefresh ? "bg-[#8A2BE2]" : "bg-slate-700"
            }`}
          >
            <div
              className={`absolute top-0.5 w-5 h-5 bg-white rounded-full shadow transition-all ${
                autoRefresh ? "left-6" : "left-0.5"
              }`}
            />
          </button>
        </div>

        {/* Refresh interval */}
        {autoRefresh && (
          <div>
            <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
              Intervalle de rafraîchissement (secondes)
            </label>
            <div className="flex items-center gap-3">
              {[1, 3, 5, 10, 30].map((s) => (
                <button
                  key={s}
                  onClick={() => setRefreshInterval(s)}
                  className={`px-3 py-1.5 rounded-lg text-sm font-bold transition-all border ${
                    refreshInterval === s
                      ? "bg-[#8A2BE2]/20 border-[#8A2BE2]/60 text-[#8A2BE2]"
                      : "bg-slate-900 border-slate-700 text-slate-400 hover:border-slate-500"
                  }`}
                >
                  {s}s
                </button>
              ))}
            </div>
          </div>
        )}
      </motion.div>

      {/* Info */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        className="glass rounded-2xl p-5 border border-white/10 max-w-2xl flex items-start gap-3"
      >
        <Info className="w-4 h-4 text-slate-500 shrink-0 mt-0.5" />
        <p className="text-xs text-slate-500">
          Les paramètres sont stockés localement dans votre navigateur. Ils
          peuvent être écrasés par les variables d&apos;environnement définies à
          la compilation (NEXT_PUBLIC_API_URL).
        </p>
      </motion.div>

      {/* Actions */}
      <div className="flex gap-3 max-w-2xl">
        <button
          onClick={handleReset}
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-white/5 hover:bg-white/10 font-bold transition-all text-sm"
        >
          <RotateCcw className="w-4 h-4" />
          Réinitialiser
        </button>
        <button
          onClick={handleSave}
          className="flex-1 px-4 py-2.5 rounded-xl bg-gradient-primary font-bold shadow-lg shadow-[#8A2BE2]/20 hover:scale-[1.02] active:scale-[0.98] transition-all text-sm flex items-center justify-center gap-2"
        >
          {saved ? (
            <>
              <CheckCircle2 className="w-4 h-4" /> Enregistré !
            </>
          ) : (
            "Sauvegarder les paramètres"
          )}
        </button>
      </div>
    </div>
  );
}

"use client";

import React, { useState } from "react";
import {
  ShieldCheck,
  Key,
  CheckCircle2,
  AlertCircle,
  Eye,
  EyeOff,
  Lock,
  Unlock,
} from "lucide-react";
import { motion } from "framer-motion";

const ENV_API_KEY = process.env.NEXT_PUBLIC_API_KEY || "";

export default function SecurityPage() {
  const [apiKey, setApiKey] = useState(() => {
    if (ENV_API_KEY) return ENV_API_KEY;
    if (typeof window !== "undefined")
      return localStorage.getItem("keryx_api_key") || "";
    return "";
  });
  const [showKey, setShowKey] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    if (typeof window !== "undefined") {
      localStorage.setItem("keryx_api_key", apiKey);
    }
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  const handleClear = () => {
    setApiKey("");
    if (typeof window !== "undefined") {
      localStorage.removeItem("keryx_api_key");
    }
  };

  const maskedKey =
    apiKey.length > 8
      ? `${apiKey.slice(0, 4)}${"•".repeat(apiKey.length - 8)}${apiKey.slice(-4)}`
      : "•".repeat(apiKey.length);

  return (
    <div className="space-y-8 pb-10">
      <div>
        <h2 className="text-4xl font-extrabold tracking-tight">Sécurité</h2>
        <p className="text-slate-400 font-medium mt-1">
          Gestion des accès et des clés API
        </p>
      </div>

      {/* API Key Status */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        className={`glass rounded-2xl p-6 border flex items-start gap-4 ${
          apiKey
            ? "border-emerald-700/40 bg-emerald-950/10"
            : "border-red-700/40 bg-red-950/10"
        }`}
      >
        {apiKey ? (
          <Lock className="w-6 h-6 text-emerald-400 shrink-0 mt-0.5" />
        ) : (
          <Unlock className="w-6 h-6 text-red-400 shrink-0 mt-0.5" />
        )}
        <div>
          <p
            className={`font-bold ${apiKey ? "text-emerald-300" : "text-red-300"}`}
          >
            {apiKey ? "API Key configurée" : "Aucune API Key configurée"}
          </p>
          <p className="text-sm text-slate-400 mt-0.5">
            {apiKey
              ? `Clé active : ${maskedKey}`
              : "Configurez une clé pour accéder aux endpoints protégés."}
          </p>
          {ENV_API_KEY && (
            <p className="text-xs text-slate-500 mt-1 flex items-center gap-1">
              <ShieldCheck className="w-3 h-3" />
              Fournie par variable d&apos;environnement (lecture seule)
            </p>
          )}
        </div>
      </motion.div>

      {/* API Key Config */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.05 }}
        className="glass rounded-3xl p-8 border border-white/10 max-w-2xl space-y-5"
      >
        <div className="flex items-center gap-3 mb-2">
          <Key className="w-5 h-5 text-[#8A2BE2]" />
          <h3 className="text-lg font-bold">API Key</h3>
        </div>

        <div>
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
            Bearer Token
          </label>
          <div className="relative group">
            <input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              disabled={!!ENV_API_KEY}
              placeholder="sk-..."
              className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl pl-4 pr-12 py-3.5 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all disabled:opacity-50 disabled:cursor-not-allowed font-mono text-sm"
            />
            <button
              type="button"
              onClick={() => setShowKey((v) => !v)}
              className="absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg hover:bg-white/10 transition-colors text-slate-500"
            >
              {showKey ? (
                <EyeOff className="w-4 h-4" />
              ) : (
                <Eye className="w-4 h-4" />
              )}
            </button>
          </div>
        </div>

        {!ENV_API_KEY && (
          <div className="flex gap-3 pt-2">
            <button
              onClick={handleClear}
              className="px-4 py-2.5 rounded-xl bg-white/5 hover:bg-white/10 font-bold transition-all text-sm"
            >
              Effacer
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
                "Sauvegarder"
              )}
            </button>
          </div>
        )}
      </motion.div>

      {/* Security notices */}
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        className="glass rounded-2xl p-6 border border-white/10 max-w-2xl space-y-4"
      >
        <h3 className="font-bold text-slate-300 flex items-center gap-2">
          <ShieldCheck className="w-5 h-5 text-[#8A2BE2]" />
          Bonnes pratiques
        </h3>
        <ul className="space-y-2.5 text-sm text-slate-400">
          {[
            "Ne partagez jamais votre clé API — elle donne accès complet à la pipeline.",
            "Préférez la variable d'environnement NEXT_PUBLIC_API_KEY pour les déploiements en production.",
            "La clé est stockée dans le localStorage du navigateur — ne l'utilisez pas sur un appareil partagé.",
            "Régénérez votre clé régulièrement pour limiter l'exposition.",
          ].map((tip, i) => (
            <li key={i} className="flex items-start gap-2.5">
              <AlertCircle className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
              {tip}
            </li>
          ))}
        </ul>
      </motion.div>
    </div>
  );
}

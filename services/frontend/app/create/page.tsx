"use client";

import React, { useState } from "react";
import {
  Play,
  Link as LinkIcon,
  AlertCircle,
  Loader2,
  CheckCircle2,
} from "lucide-react";

type Job = {
  id: string;
  source_url: string;
  target_langs: string[];
  status: string;
};

interface CreateJobPayload {
  video_url: string;
  target_langs: string[];
  prompt?: string;
  lora?: string;
}

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "https://orchestrator.p.zacharie.org";
const ENV_API_KEY = process.env.NEXT_PUBLIC_API_KEY || "";

const LANGS = ["fr", "en", "es", "de", "it", "ja", "zh", "ar", "pt"];

export default function CreatePage() {
  const [videoUrl, setVideoUrl] = useState("");
  const [selectedLangs, setSelectedLangs] = useState<string[]>(["fr"]);
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState(() => {
    if (ENV_API_KEY) return ENV_API_KEY;
    if (typeof window !== "undefined")
      return localStorage.getItem("keryx_api_key") || "";
    return "";
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [createdJob, setCreatedJob] = useState<Job | null>(null);

  const toggleLang = (lang: string) => {
    setSelectedLangs((prev) =>
      prev.includes(lang) ? prev.filter((l) => l !== lang) : [...prev, lang]
    );
  };

  const handleSubmit = async () => {
    if (!videoUrl.trim()) {
      setError("L'URL de la vidéo est requise.");
      return;
    }
    if (selectedLangs.length === 0) {
      setError("Sélectionnez au moins une langue.");
      return;
    }
    const effectiveKey = ENV_API_KEY || apiKey;
    if (!effectiveKey.trim()) {
      setError("Une API Key est requise.");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const payload: CreateJobPayload = {
        video_url: videoUrl,
        target_langs: selectedLangs,
        ...(prompt ? { prompt } : {}),
      };
      const res = await fetch(`${API_BASE}/api/jobs`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${effectiveKey}`,
        },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: res.statusText }));
        throw new Error(err.error || `HTTP ${res.status}`);
      }
      const { job_id } = await res.json();
      const jobRes = await fetch(`${API_BASE}/api/jobs/${job_id}`);
      const job: Job = await jobRes.json();
      setCreatedJob(job);
      setVideoUrl("");
      setPrompt("");
      setSelectedLangs(["fr"]);
      if (!ENV_API_KEY && typeof window !== "undefined") {
        localStorage.setItem("keryx_api_key", apiKey);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-8 pb-10">
      <div>
        <h2 className="text-4xl font-extrabold tracking-tight">Créer un Job</h2>
        <p className="text-slate-400 font-medium mt-1">
          Soumettez une vidéo pour transcription et doublage automatique
        </p>
      </div>

      {createdJob && (
        <div className="glass rounded-2xl p-5 border border-emerald-700/40 bg-emerald-950/20 flex items-start gap-4">
          <CheckCircle2 className="w-6 h-6 text-emerald-400 shrink-0 mt-0.5" />
          <div>
            <p className="font-bold text-emerald-300">Job créé avec succès !</p>
            <p className="text-sm text-slate-400 font-mono mt-1">
              ID : {createdJob.id}
            </p>
            <p className="text-sm text-slate-500 mt-0.5 truncate">
              {createdJob.source_url}
            </p>
          </div>
        </div>
      )}

      <div className="glass rounded-3xl p-8 border border-white/10 max-w-2xl space-y-6">
        {/* URL */}
        <div>
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
            URL Source
          </label>
          <div className="relative group">
            <LinkIcon className="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500 group-focus-within:text-[#8A2BE2] transition-colors" />
            <input
              type="url"
              placeholder="https://youtube.com/watch?v=..."
              value={videoUrl}
              onChange={(e) => setVideoUrl(e.target.value)}
              className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl pl-11 pr-4 py-3.5 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all"
            />
          </div>
        </div>

        {/* Langues */}
        <div>
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
            Langues cibles ({selectedLangs.length} sélectionnée
            {selectedLangs.length > 1 ? "s" : ""})
          </label>
          <div className="flex flex-wrap gap-2">
            {LANGS.map((lang) => (
              <button
                key={lang}
                onClick={() => toggleLang(lang)}
                className={`px-3 py-1.5 rounded-lg text-sm font-bold uppercase tracking-wide transition-all border ${selectedLangs.includes(lang)
                    ? "bg-[#8A2BE2]/20 border-[#8A2BE2]/60 text-[#8A2BE2]"
                    : "bg-slate-900 border-slate-700 text-slate-400 hover:border-slate-500"
                  }`}
              >
                {lang}
              </button>
            ))}
          </div>
        </div>

        {/* Prompt */}
        <div>
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
            Prompt style{" "}
            <span className="text-slate-600 normal-case font-normal">
              (optionnel)
            </span>
          </label>
          <textarea
            rows={2}
            placeholder="Modern professional SaaS presentation..."
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl px-4 py-3 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all text-sm resize-none"
          />
        </div>

        {/* API Key */}
        {!ENV_API_KEY && (
          <div>
            <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
              API Key
            </label>
            <input
              type="password"
              placeholder="Bearer token..."
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl px-4 py-3 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all text-sm"
            />
          </div>
        )}

        {error && (
          <p className="text-red-400 text-sm flex items-center gap-2">
            <AlertCircle className="w-4 h-4" /> {error}
          </p>
        )}

        <button
          onClick={handleSubmit}
          disabled={loading}
          className="w-full px-4 py-3.5 rounded-xl bg-gradient-primary font-bold shadow-lg shadow-[#8A2BE2]/20 hover:scale-[1.02] active:scale-[0.98] transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Play className="w-4 h-4" />
          )}
          {loading ? "Soumission en cours..." : "Démarrer le job"}
        </button>
      </div>
    </div>
  );
}

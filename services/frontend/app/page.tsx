"use client";

import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Plus,
  Search,
  Zap,
  Clock,
  CheckCircle2,
  AlertCircle,
  Link as LinkIcon,
  Play,
  Terminal,
  Server,
  Globe,
  X,
  RefreshCw,
  ChevronRight,
  Activity,
  Loader2,
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

// ─────────────────────────────────────────────
// Types matching the real Rust backend
// ─────────────────────────────────────────────
type JobStatus =
  | "Pending"
  | "Processing"
  | { Processing: string }
  | "Completed"
  | { Failed: string };

interface StyleConfig {
  prompt: string;
  lora: string | null;
}

interface AssetMap {
  lang: string;
  s3_key: string;
}

interface Job {
  id: string;
  source_url: string;
  target_langs: string[];
  status: JobStatus;
  style_config: StyleConfig;
  assets_map: AssetMap[];
}

interface CreateJobPayload {
  video_url: string;
  target_langs: string[];
  prompt?: string;
  lora?: string;
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────
const API_BASE = process.env.NEXT_PUBLIC_API_URL || "https://ingestor.p.zacharie.org";
const ENV_API_KEY = process.env.NEXT_PUBLIC_API_KEY || "";

function getStatusLabel(status: JobStatus): string {
  if (status === "Pending") return "pending";
  if (status === "Completed") return "completed";
  if (typeof status === "object" && "Failed" in status) return "failed";
  if (status === "Processing" || (typeof status === "object" && "Processing" in status))
    return "processing";
  return "pending";
}

function getStatusStep(status: JobStatus): string {
  if (typeof status === "object" && "Processing" in status) return status.Processing;
  return "";
}

// ─────────────────────────────────────────────
// Subcomponents
// ─────────────────────────────────────────────
const StatusBadge = ({ status }: { status: JobStatus }) => {
  const label = getStatusLabel(status);
  const configs: Record<string, { bg: string; text: string; icon: React.ElementType }> = {
    pending: { bg: "bg-slate-800 border-slate-700", text: "text-slate-400", icon: Clock },
    processing: { bg: "bg-blue-950 border-blue-700", text: "text-blue-300", icon: Zap },
    completed: { bg: "bg-emerald-950 border-emerald-700", text: "text-emerald-300", icon: CheckCircle2 },
    failed: { bg: "bg-red-950 border-red-700", text: "text-red-400", icon: AlertCircle },
  };
  const cfg = configs[label] || configs.pending;
  const Icon = cfg.icon;
  return (
    <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider border ${cfg.bg} ${cfg.text}`}>
      <Icon className="w-3 h-3" />
      {label}
    </span>
  );
};

const StatCard = ({
  title,
  value,
  icon: Icon,
  colorClass,
}: {
  title: string;
  value: number | string;
  icon: React.ElementType;
  colorClass: string;
}) => (
  <div className="glass rounded-2xl p-6 relative overflow-hidden group">
    <div className={`absolute top-0 right-0 w-24 h-24 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2 opacity-20 group-hover:opacity-30 transition-all ${colorClass}`} />
    <div className="flex items-center justify-between mb-4">
      <div className={`p-3 rounded-xl ${colorClass} bg-opacity-10`}>
        <Icon className="w-6 h-6" />
      </div>
    </div>
    <p className="text-slate-400 text-sm font-medium">{title}</p>
    <h3 className="text-3xl font-bold mt-1">{value}</h3>
  </div>
);

// ─────────────────────────────────────────────
// Log Drawer
// ─────────────────────────────────────────────
function LogDrawer({ job, apiKey, onClose }: { job: Job; apiKey: string; onClose: () => void }) {
  const [logs, setLogs] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const url = `${API_BASE}/api/jobs/${job.id}/logs`;
    const es = new EventSource(url);

    es.addEventListener("log", (e) => {
      setLogs((prev) => [...prev, ...e.data.split("\n")]);
    });
    es.addEventListener("done", () => {
      setDone(true);
      es.close();
    });
    es.addEventListener("error", () => {
      setLogs((prev) => [...prev, "[SSE disconnected]"]);
      es.close();
    });
    return () => es.close();
  }, [job.id]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <motion.div
      initial={{ x: "100%" }}
      animate={{ x: 0 }}
      exit={{ x: "100%" }}
      transition={{ type: "spring", damping: 25 }}
      className="fixed inset-y-0 right-0 w-full max-w-2xl z-50 glass border-l border-white/10 flex flex-col"
    >
      <div className="flex items-center justify-between p-6 border-b border-white/10">
        <div>
          <h3 className="font-bold text-lg flex items-center gap-2">
            <Terminal className="w-5 h-5 text-primary" />
            Live Logs — Job {job.id.slice(0, 8)}
          </h3>
          <p className="text-xs text-slate-400 truncate mt-1">{job.source_url}</p>
        </div>
        <div className="flex items-center gap-3">
          <StatusBadge status={job.status} />
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-white/10 transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-6 font-mono text-[12px] space-y-0.5">
        {logs.length === 0 && !done && (
          <div className="flex items-center gap-2 text-slate-500 animate-pulse">
            <Loader2 className="w-4 h-4 animate-spin" />
            Waiting for logs...
          </div>
        )}
        {logs.map((line, i) => (
          <p
            key={i}
            className={
              line.includes("ERROR") || line.includes("error")
                ? "text-red-400"
                : line.includes("warn") || line.includes("WARN")
                ? "text-amber-400"
                : line.includes("✅") || line.includes("completed")
                ? "text-emerald-400"
                : "text-slate-300"
            }
          >
            {line}
          </p>
        ))}
        {done && (
          <p className="text-emerald-400 font-bold mt-2">
            ✅ Job finished
          </p>
        )}
        <div ref={bottomRef} />
      </div>
    </motion.div>
  );
}

// ─────────────────────────────────────────────
// Create Job Modal
// ─────────────────────────────────────────────
const LANGS = ["fr", "en", "es", "de", "it", "ja", "zh", "ar", "pt"];

function CreateModal({
  onClose,
  onCreated,
  apiKey,
  setApiKey,
}: {
  onClose: () => void;
  onCreated: (job: Job) => void;
  apiKey: string;
  setApiKey: (v: string) => void;
}) {
  const [videoUrl, setVideoUrl] = useState("");
  const [selectedLangs, setSelectedLangs] = useState<string[]>(["fr"]);
  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const toggleLang = (lang: string) => {
    setSelectedLangs((prev) =>
      prev.includes(lang) ? prev.filter((l) => l !== lang) : [...prev, lang]
    );
  };

  const handleSubmit = async () => {
    if (!videoUrl.trim()) { setError("L'URL de la vidéo est requise."); return; }
    if (selectedLangs.length === 0) { setError("Sélectionnez au moins une langue."); return; }
    const effectiveKey = ENV_API_KEY || apiKey;
    if (!effectiveKey.trim()) { setError("Une API Key est requise."); return; }
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
      // Fetch the job to get full details
      const jobRes = await fetch(`${API_BASE}/api/jobs/${job_id}`);
      const job: Job = await jobRes.json();
      onCreated(job);
      onClose();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        onClick={onClose}
        className="absolute inset-0 bg-black/60 backdrop-blur-md"
      />
      <motion.div
        initial={{ scale: 0.9, opacity: 0, y: 20 }}
        animate={{ scale: 1, opacity: 1, y: 0 }}
        exit={{ scale: 0.9, opacity: 0, y: 20 }}
        className="glass max-w-lg w-full p-8 rounded-3xl relative z-10 border border-white/10"
      >
        <button onClick={onClose} className="absolute top-4 right-4 p-2 hover:bg-white/10 rounded-lg transition-colors">
          <X className="w-4 h-4" />
        </button>
        <h3 className="text-2xl font-bold mb-1">Nouveau Job</h3>
        <p className="text-slate-400 text-sm mb-6">Soumettez une vidéo à transcrire et dubber</p>

        <div className="space-y-5">
          {/* URL */}
          <div>
            <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">URL Source</label>
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
              Langues cibles ({selectedLangs.length} sélectionnée{selectedLangs.length > 1 ? "s" : ""})
            </label>
            <div className="flex flex-wrap gap-2">
              {LANGS.map((lang) => (
                <button
                  key={lang}
                  onClick={() => toggleLang(lang)}
                  className={`px-3 py-1.5 rounded-lg text-sm font-bold uppercase tracking-wide transition-all border ${
                    selectedLangs.includes(lang)
                      ? "bg-[#8A2BE2]/20 border-[#8A2BE2]/60 text-[#8A2BE2]"
                      : "bg-slate-900 border-slate-700 text-slate-400 hover:border-slate-500"
                  }`}
                >
                  {lang}
                </button>
              ))}
            </div>
          </div>

          {/* Prompt optionnel */}
          <div>
            <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">
              Prompt style <span className="text-slate-600 normal-case font-normal">(optionnel)</span>
            </label>
            <textarea
              rows={2}
              placeholder="Modern professional SaaS presentation..."
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl px-4 py-3 text-slate-200 focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all text-sm resize-none"
            />
          </div>

          {/* API Key — hidden if provided by env */}
          {!ENV_API_KEY && (
            <div>
              <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">API Key</label>
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

          <div className="pt-2 flex gap-3">
            <button
              onClick={onClose}
              className="flex-1 px-4 py-3 rounded-xl bg-white/5 hover:bg-white/10 font-bold transition-all"
            >
              Annuler
            </button>
            <button
              onClick={handleSubmit}
              disabled={loading}
              className="flex-1 px-4 py-3 rounded-xl bg-gradient-primary font-bold shadow-lg shadow-[#8A2BE2]/20 hover:scale-[1.02] active:scale-[0.98] transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
              {loading ? "Soumission..." : "Démarrer"}
            </button>
          </div>
        </div>
      </motion.div>
    </div>
  );
}

// ─────────────────────────────────────────────
// Job Card
// ─────────────────────────────────────────────
function JobCard({ job, onClick }: { job: Job; onClick: () => void }) {
  const label = getStatusLabel(job.status);
  const step = getStatusStep(job.status);
  const isProcessing = label === "processing";

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      onClick={onClick}
      className="glass hover:bg-white/[0.04] transition-all p-5 rounded-2xl border border-white/5 group cursor-pointer"
    >
      <div className="flex items-center gap-4">
        <div className="w-14 h-14 rounded-xl bg-slate-800 border border-slate-700 flex items-center justify-center shrink-0 relative overflow-hidden">
          <Globe className="w-7 h-7 text-slate-600" />
          {isProcessing && (
            <div className="absolute inset-0 flex items-center justify-center bg-slate-900/80">
              <Loader2 className="w-5 h-5 text-[#8A2BE2] animate-spin" />
            </div>
          )}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-3 mb-1 flex-wrap">
            <h4 className="font-bold truncate text-base">{job.source_url}</h4>
            <StatusBadge status={job.status} />
          </div>
          <div className="flex items-center gap-4 text-xs text-slate-500 font-medium flex-wrap">
            <span className="flex items-center gap-1">
              <Globe className="w-3 h-3" />
              {job.target_langs.join(", ")}
            </span>
            <span className="font-mono text-[10px] text-slate-600">
              {job.id.slice(0, 8)}
            </span>
            {step && (
              <span className="text-blue-400 truncate max-w-[200px]">{step}</span>
            )}
          </div>
        </div>
        <div className="ml-2 opacity-0 group-hover:opacity-100 transition-opacity">
          <ChevronRight className="w-5 h-5 text-slate-500" />
        </div>
      </div>

      {isProcessing && (
        <div className="mt-4 pt-4 border-t border-white/5">
          <div className="h-1 w-full bg-slate-800 rounded-full overflow-hidden">
            <motion.div
              animate={{ x: ["−100%", "100%"] }}
              transition={{ repeat: Infinity, duration: 1.5, ease: "linear" }}
              className="h-full w-1/3 bg-gradient-primary rounded-full"
            />
          </div>
        </div>
      )}
    </motion.div>
  );
}

// ─────────────────────────────────────────────
// Main Dashboard
// ─────────────────────────────────────────────
export default function DashboardPage() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [selectedJob, setSelectedJob] = useState<Job | null>(null);
  const [apiKey, setApiKey] = useState(() => {
    if (ENV_API_KEY) return ENV_API_KEY;
    if (typeof window !== "undefined") return localStorage.getItem("keryx_api_key") || "";
    return "";
  });

  // Persist API key (only if coming from user input, not env)
  useEffect(() => {
    if (ENV_API_KEY) return;
    if (typeof window !== "undefined") localStorage.setItem("keryx_api_key", apiKey);
  }, [apiKey]);

  const fetchJob = useCallback(async (id: string): Promise<Job | null> => {
    try {
      const res = await fetch(`${API_BASE}/api/jobs/${id}`);
      if (!res.ok) return null;
      return res.json();
    } catch {
      return null;
    }
  }, []);

  const refreshJobs = useCallback(async () => {
    if (jobs.length === 0) return;
    const updated = await Promise.all(jobs.map((j) => fetchJob(j.id)));
    setJobs((prev) =>
      prev.map((j, i) => updated[i] || j)
    );
    // Refresh selected job
    if (selectedJob) {
      const fresh = await fetchJob(selectedJob.id);
      if (fresh) setSelectedJob(fresh);
    }
  }, [jobs, selectedJob, fetchJob]);

  // Auto-refresh when processing jobs exist
  useEffect(() => {
    const hasActive = jobs.some((j) => getStatusLabel(j.status) === "processing");
    if (!hasActive) return;
    const interval = setInterval(refreshJobs, 3000);
    return () => clearInterval(interval);
  }, [jobs, refreshJobs]);

  const handleJobCreated = (job: Job) => {
    setJobs((prev) => [job, ...prev]);
    setSelectedJob(job);
  };

  const filteredJobs = jobs.filter(
    (j) =>
      j.source_url.toLowerCase().includes(search.toLowerCase()) ||
      j.id.toLowerCase().includes(search.toLowerCase()) ||
      j.target_langs.some((l) => l.includes(search.toLowerCase()))
  );

  const stats = {
    total: jobs.length,
    active: jobs.filter((j) => getStatusLabel(j.status) === "processing").length,
    done: jobs.filter((j) => getStatusLabel(j.status) === "completed").length,
    failed: jobs.filter((j) => getStatusLabel(j.status) === "failed").length,
  };

  return (
    <div className="space-y-8 pb-10">
      {/* Header */}
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h2 className="text-4xl font-extrabold tracking-tight">Vue d&apos;ensemble</h2>
          <p className="text-slate-400 font-medium">Pipeline IA de dubbingK vidéo distribué</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={refreshJobs}
            disabled={loading}
            className="p-2.5 glass rounded-xl hover:bg-white/10 transition-colors"
            title="Refresh"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
          </button>
          <button
            onClick={() => setIsCreating(true)}
            className="bg-gradient-primary hover:scale-105 transition-transform px-6 py-3 rounded-xl font-bold flex items-center gap-2 shadow-lg shadow-[#8A2BE2]/20"
          >
            <Plus className="w-5 h-5" />
            Nouveau Job
          </button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="Total Jobs" value={stats.total} icon={Server} colorClass="bg-[#8A2BE2] text-[#8A2BE2]" />
        <StatCard title="En cours" value={stats.active} icon={Zap} colorClass="bg-blue-500 text-blue-400" />
        <StatCard title="Complétés" value={stats.done} icon={CheckCircle2} colorClass="bg-emerald-500 text-emerald-400" />
        <StatCard title="Échoués" value={stats.failed} icon={AlertCircle} colorClass="bg-red-500 text-red-400" />
      </div>

      {/* Jobs List */}
      <div className="space-y-4">
        <div className="flex items-center justify-between flex-wrap gap-3">
          <h3 className="text-xl font-bold flex items-center gap-2">
            <Activity className="text-[#8A2BE2] w-5 h-5" />
            Jobs
          </h3>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
            <input
              type="text"
              placeholder="Rechercher URL, ID, langue..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="bg-white/5 border border-slate-700/50 rounded-lg pl-9 pr-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[#8A2BE2]/50 transition-all w-64"
            />
          </div>
        </div>

        <AnimatePresence mode="popLayout">
          {filteredJobs.length === 0 ? (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="glass rounded-2xl p-16 text-center"
            >
              <Server className="w-12 h-12 text-slate-700 mx-auto mb-4" />
              <p className="text-slate-500 font-medium">Aucun job. Cliquez sur &quot;Nouveau Job&quot; pour commencer.</p>
            </motion.div>
          ) : (
            filteredJobs.map((job) => (
              <JobCard
                key={job.id}
                job={job}
                onClick={() => setSelectedJob(job)}
              />
            ))
          )}
        </AnimatePresence>
      </div>

      {/* Modals */}
      <AnimatePresence>
        {isCreating && (
          <CreateModal
            onClose={() => setIsCreating(false)}
            onCreated={handleJobCreated}
            apiKey={apiKey}
            setApiKey={setApiKey}
          />
        )}
        {selectedJob && (
          <LogDrawer
            job={selectedJob}
            apiKey={apiKey}
            onClose={() => setSelectedJob(null)}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

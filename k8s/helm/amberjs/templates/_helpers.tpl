{{/*
Expand the name of the chart.
*/}}
{{- define "amberjs.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "amberjs.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "amberjs.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "amberjs.labels" -}}
helm.sh/chart: {{ include "amberjs.chart" . }}
{{ include "amberjs.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "amberjs.selectorLabels" -}}
app.kubernetes.io/name: {{ include "amberjs.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "amberjs.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "amberjs.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Amber specific labels
*/}}
{{- define "amberjs.component" -}}
app.kubernetes.io/component: runtime
{{- end }}

{{/*
Performance configuration
*/}}
{{- define "amberjs.performance.config" -}}
{{- $config := dict -}}
{{- $_ := set $config "jit" .Values.performance.jitOptimization -}}
{{- $_ := set $config "zeroCopyIO" .Values.performance.zeroCopyIO -}}
{{- $_ := set $config "memoryPool" .Values.performance.memoryPool.enabled -}}
{{- toYaml $config }}
{{- end }}

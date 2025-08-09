/*
 * garguile - guile bindings for rust
 * Copyright (C) 2025  Andrew Chi
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include "reexports.h"

const SCM GARGUILE_REEXPORTS_SCM_BOOL_T = SCM_BOOL_T;
const SCM GARGUILE_REEXPORTS_SCM_BOOL_F = SCM_BOOL_F;
const SCM GARGUILE_REEXPORTS_SCM_EOL = SCM_EOL;
const SCM GARGUILE_REEXPORTS_SCM_UNDEFINED = SCM_UNDEFINED;

const int GARGUILE_REEXPORTS_SCM_F_DYNWIND_REWINDABLE = SCM_F_DYNWIND_REWINDABLE;
const int GARGUILE_REEXPORTS_SCM_F_WIND_EXPLICITLY = SCM_F_WIND_EXPLICITLY;

int garguile_reexports_scm_is_true(SCM b) {
  return scm_is_true(b);
}
int garguile_reexports_scm_is_false(SCM b) {
  return scm_is_false(b);
}

int GARGUILE_REEXPORTS_SCM_HOOK_ARITY(SCM hook) {
  return SCM_HOOK_ARITY(hook);
}

int GARGUILE_REEXPORTS_SCM_HOOKP(SCM hook) {
  return SCM_HOOKP(hook);
}
int GARGUILE_REEXPORTS_SCM_MODULEP(SCM obj) {
  return SCM_MODULEP(obj);
}
int GARGUILE_REEXPORTS_SCM_IS_A_P(SCM val, SCM ty) {
  return SCM_IS_A_P(val, ty);
}
int GARGUILE_REEXPORTS_SCM_UNBNDP(SCM scm) {
  return SCM_UNBNDP(scm);
}

uintptr_t garguile_reexports_scm_to_uintptr_t(SCM scm) {
  return scm_to_uintptr_t(scm);
}
intptr_t garguile_reexports_scm_to_intptr_t(SCM scm) {
  return scm_to_intptr_t(scm);
}

SCM garguile_reexports_scm_from_uintptr_t(uintptr_t i) {
  return scm_from_uintptr_t(i);
}
SCM garguile_reexports_scm_from_intptr_t(intptr_t i) {
  return scm_from_intptr_t(i);
}

/*
 * gargoyle - guile bindings for rust
 * Copyright (C) 2025  Andrew Chi
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

#include "reexports.h"

const SCM GARGOYLE_REEXPORTS_SCM_BOOL_T = SCM_BOOL_T;
const SCM GARGOYLE_REEXPORTS_SCM_BOOL_F = SCM_BOOL_F;
const SCM GARGOYLE_REEXPORTS_SCM_UNDEFINED = SCM_UNDEFINED;

_Bool gargoyle_reexports_scm_is_true(SCM b) {
  return scm_is_true(b);
}
_Bool GARGOYLE_REEXPORTS_SCM_UNBNDP(SCM scm) {
  return SCM_UNBNDP(scm);
}

uintptr_t gargoyle_reexports_scm_to_uintptr_t(SCM scm) {
  return scm_to_uintptr_t(scm);
}
intptr_t gargoyle_reexports_scm_to_intptr_t(SCM scm) {
  return scm_to_intptr_t(scm);
}

SCM gargoyle_reexports_scm_from_uintptr_t(uintptr_t i) {
  return scm_from_uintptr_t(i);
}
SCM gargoyle_reexports_scm_from_intptr_t(intptr_t i) {
  return scm_from_intptr_t(i);
}

/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/**
 * This file is near-OBSOLETE. It is used for about:blank only and for the
 * HTML element factory.
 * Don't bother adding new stuff in this file.
 */

#include "mozilla/ArrayUtils.h"

#include "nsContentSink.h"
#include "nsCOMPtr.h"
#include "nsHTMLTags.h"
#include "nsReadableUtils.h"
#include "nsUnicharUtils.h"
#include "nsIHTMLContentSink.h"
#include "nsIInterfaceRequestor.h"
#include "nsIInterfaceRequestorUtils.h"
#include "nsIURI.h"
#include "mozilla/dom/NodeInfo.h"
#include "mozilla/dom/ScriptLoader.h"
#include "nsCRT.h"
#include "prtime.h"
#include "mozilla/Logging.h"
#include "nsIContent.h"
#include "mozilla/dom/CustomElementRegistry.h"
#include "mozilla/dom/Element.h"
#include "mozilla/dom/MutationObservers.h"
#include "mozilla/Preferences.h"

#include "nsGenericHTMLElement.h"

#include "nsIScriptElement.h"

#include "nsDocElementCreatedNotificationRunner.h"
#include "nsGkAtoms.h"
#include "nsContentUtils.h"
#include "nsIChannel.h"
#include "mozilla/dom/Document.h"
#include "nsStubDocumentObserver.h"
#include "nsHTMLDocument.h"
#include "nsTArray.h"
#include "nsTextFragment.h"
#include "nsIScriptGlobalObject.h"
#include "nsNameSpaceManager.h"

#include "nsError.h"
#include "nsContentPolicyUtils.h"
#include "nsIDocShell.h"
#include "nsIScriptContext.h"

#include "nsLayoutCID.h"

#include "nsEscape.h"
#include "nsNodeInfoManager.h"
#include "nsContentCreatorFunctions.h"
#include "mozAutoDocUpdate.h"
#include "nsTextNode.h"

using namespace mozilla;
using namespace mozilla::dom;

//----------------------------------------------------------------------

nsGenericHTMLElement* NS_NewHTMLNOTUSEDElement(
    already_AddRefed<mozilla::dom::NodeInfo>&& aNodeInfo,
    FromParser aFromParser) {
  MOZ_ASSERT_UNREACHABLE("The element ctor should never be called");
  return nullptr;
}

#define HTML_TAG(_tag, _classname, _interfacename) \
  NS_NewHTML##_classname##Element,
#define HTML_OTHER(_tag) NS_NewHTMLNOTUSEDElement,
static const HTMLContentCreatorFunction sHTMLContentCreatorFunctions[] = {
    NS_NewHTMLUnknownElement,
#include "nsHTMLTagList.h"
#undef HTML_TAG
#undef HTML_OTHER
    NS_NewHTMLUnknownElement};

nsresult NS_NewHTMLElement(Element** aResult,
                           already_AddRefed<mozilla::dom::NodeInfo>&& aNodeInfo,
                           FromParser aFromParser, nsAtom* aIsAtom,
                           mozilla::dom::CustomElementDefinition* aDefinition) {
  RefPtr<mozilla::dom::NodeInfo> nodeInfo = aNodeInfo;

  NS_ASSERTION(
      nodeInfo->NamespaceEquals(kNameSpaceID_XHTML),
      "Trying to create HTML elements that don't have the XHTML namespace");

  return nsContentUtils::NewXULOrHTMLElement(aResult, nodeInfo, aFromParser,
                                             aIsAtom, aDefinition);
}

already_AddRefed<nsGenericHTMLElement> CreateHTMLElement(
    uint32_t aNodeType, already_AddRefed<mozilla::dom::NodeInfo>&& aNodeInfo,
    FromParser aFromParser) {
  NS_ASSERTION(
      aNodeType <= NS_HTML_TAG_MAX || aNodeType == eHTMLTag_userdefined,
      "aNodeType is out of bounds");

  HTMLContentCreatorFunction cb = sHTMLContentCreatorFunctions[aNodeType];

  NS_ASSERTION(cb != NS_NewHTMLNOTUSEDElement,
               "Don't know how to construct tag element!");

  RefPtr<nsGenericHTMLElement> result = cb(std::move(aNodeInfo), aFromParser);

  return result.forget();
}

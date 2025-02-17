// This file was autogenerated by binjs_generate_spidermonkey,
// please DO NOT EDIT BY HAND.
/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*-
 * vim: set ts=8 sts=2 et sw=2 tw=80:
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// To generate this file, see the documentation in
// js/src/frontend/binsource/README.md.

#ifndef frontend_BinToken_h
#define frontend_BinToken_h

#include <stddef.h>

/**
 * Definition of Binary AST tokens.
 *
 * In the Binary AST world, an AST is composed of nodes, where a node is
 * defined by:
 * - a Kind (see `BinKind`);
 * - a list of fields, where each field is:
 *    - a Name (see `BinField`);
 *    - a Value, which may be either a node or a primitive value.
 *
 * The mapping between Kind and list of fields is determined entirely by
 * the grammar of Binary AST. The mapping between (Kind, Name) and the
 * structure of Value is also determined entirely by the grammar of
 * Binary AST.
 *
 * As per the specifications of Binary AST, kinds may be added as the
 * language grows, but never removed. The mapping between Kind and list
 * of fields may also change to add new fields or make some fields optional,
 * but may never remove a field. Finally, the mapping between (Kind, Name)
 * and the structure of Value may be modified to add new possible values,
 * but never to remove a value.
 *
 * A Binary AST parser must be able to fail gracefully when confronted with
 * unknown Kinds or Names.
 */

namespace js {
namespace frontend {

/**
 * The different kinds of Binary AST nodes, as per the specifications of
 * Binary AST.
 *
 * These kinds match roughly with the `ParseNodeKind` used internally.
 *
 * Usage:
 *
 * ```c++
 * #define WITH_KIND(CPP_NAME, SPEC_NAME) ...
 * FOR_EACH_BIN_KIND(WITH_KIND)
 * ```
 *
 *
 * (sorted by alphabetical order)
 */
#define FOR_EACH_BIN_KIND(F)                                                  \
  F(_Null, "")                                                                \
  F(ArrayAssignmentTarget, "ArrayAssignmentTarget")                           \
  F(ArrayBinding, "ArrayBinding")                                             \
  F(ArrayExpression, "ArrayExpression")                                       \
  F(ArrowExpressionContentsWithExpression,                                    \
    "ArrowExpressionContentsWithExpression")                                  \
  F(ArrowExpressionContentsWithFunctionBody,                                  \
    "ArrowExpressionContentsWithFunctionBody")                                \
  F(AssertedBlockScope, "AssertedBlockScope")                                 \
  F(AssertedBoundName, "AssertedBoundName")                                   \
  F(AssertedBoundNamesScope, "AssertedBoundNamesScope")                       \
  F(AssertedDeclaredName, "AssertedDeclaredName")                             \
  F(AssertedParameterName, "AssertedParameterName")                           \
  F(AssertedParameterScope, "AssertedParameterScope")                         \
  F(AssertedPositionalParameterName, "AssertedPositionalParameterName")       \
  F(AssertedRestParameterName, "AssertedRestParameterName")                   \
  F(AssertedScriptGlobalScope, "AssertedScriptGlobalScope")                   \
  F(AssertedVarScope, "AssertedVarScope")                                     \
  F(AssignmentExpression, "AssignmentExpression")                             \
  F(AssignmentTargetIdentifier, "AssignmentTargetIdentifier")                 \
  F(AssignmentTargetPropertyIdentifier, "AssignmentTargetPropertyIdentifier") \
  F(AssignmentTargetPropertyProperty, "AssignmentTargetPropertyProperty")     \
  F(AssignmentTargetWithInitializer, "AssignmentTargetWithInitializer")       \
  F(AwaitExpression, "AwaitExpression")                                       \
  F(BinaryExpression, "BinaryExpression")                                     \
  F(BindingIdentifier, "BindingIdentifier")                                   \
  F(BindingPropertyIdentifier, "BindingPropertyIdentifier")                   \
  F(BindingPropertyProperty, "BindingPropertyProperty")                       \
  F(BindingWithInitializer, "BindingWithInitializer")                         \
  F(Block, "Block")                                                           \
  F(BreakStatement, "BreakStatement")                                         \
  F(CallExpression, "CallExpression")                                         \
  F(CatchClause, "CatchClause")                                               \
  F(ClassDeclaration, "ClassDeclaration")                                     \
  F(ClassElement, "ClassElement")                                             \
  F(ClassExpression, "ClassExpression")                                       \
  F(CompoundAssignmentExpression, "CompoundAssignmentExpression")             \
  F(ComputedMemberAssignmentTarget, "ComputedMemberAssignmentTarget")         \
  F(ComputedMemberExpression, "ComputedMemberExpression")                     \
  F(ComputedPropertyName, "ComputedPropertyName")                             \
  F(ConditionalExpression, "ConditionalExpression")                           \
  F(ContinueStatement, "ContinueStatement")                                   \
  F(DataProperty, "DataProperty")                                             \
  F(DebuggerStatement, "DebuggerStatement")                                   \
  F(Directive, "Directive")                                                   \
  F(DoWhileStatement, "DoWhileStatement")                                     \
  F(EagerArrowExpressionWithExpression, "EagerArrowExpressionWithExpression") \
  F(EagerArrowExpressionWithFunctionBody,                                     \
    "EagerArrowExpressionWithFunctionBody")                                   \
  F(EagerFunctionDeclaration, "EagerFunctionDeclaration")                     \
  F(EagerFunctionExpression, "EagerFunctionExpression")                       \
  F(EagerGetter, "EagerGetter")                                               \
  F(EagerMethod, "EagerMethod")                                               \
  F(EagerSetter, "EagerSetter")                                               \
  F(EmptyStatement, "EmptyStatement")                                         \
  F(Export, "Export")                                                         \
  F(ExportAllFrom, "ExportAllFrom")                                           \
  F(ExportDefault, "ExportDefault")                                           \
  F(ExportFrom, "ExportFrom")                                                 \
  F(ExportFromSpecifier, "ExportFromSpecifier")                               \
  F(ExportLocalSpecifier, "ExportLocalSpecifier")                             \
  F(ExportLocals, "ExportLocals")                                             \
  F(ExpressionStatement, "ExpressionStatement")                               \
  F(ForInOfBinding, "ForInOfBinding")                                         \
  F(ForInStatement, "ForInStatement")                                         \
  F(ForOfStatement, "ForOfStatement")                                         \
  F(ForStatement, "ForStatement")                                             \
  F(FormalParameters, "FormalParameters")                                     \
  F(FunctionExpressionContents, "FunctionExpressionContents")                 \
  F(FunctionOrMethodContents, "FunctionOrMethodContents")                     \
  F(GetterContents, "GetterContents")                                         \
  F(IdentifierExpression, "IdentifierExpression")                             \
  F(IfStatement, "IfStatement")                                               \
  F(Import, "Import")                                                         \
  F(ImportNamespace, "ImportNamespace")                                       \
  F(ImportSpecifier, "ImportSpecifier")                                       \
  F(LabelledStatement, "LabelledStatement")                                   \
  F(LazyArrowExpressionWithExpression, "LazyArrowExpressionWithExpression")   \
  F(LazyArrowExpressionWithFunctionBody,                                      \
    "LazyArrowExpressionWithFunctionBody")                                    \
  F(LazyFunctionDeclaration, "LazyFunctionDeclaration")                       \
  F(LazyFunctionExpression, "LazyFunctionExpression")                         \
  F(LazyGetter, "LazyGetter")                                                 \
  F(LazyMethod, "LazyMethod")                                                 \
  F(LazySetter, "LazySetter")                                                 \
  F(LiteralBooleanExpression, "LiteralBooleanExpression")                     \
  F(LiteralInfinityExpression, "LiteralInfinityExpression")                   \
  F(LiteralNullExpression, "LiteralNullExpression")                           \
  F(LiteralNumericExpression, "LiteralNumericExpression")                     \
  F(LiteralPropertyName, "LiteralPropertyName")                               \
  F(LiteralRegExpExpression, "LiteralRegExpExpression")                       \
  F(LiteralStringExpression, "LiteralStringExpression")                       \
  F(Module, "Module")                                                         \
  F(NewExpression, "NewExpression")                                           \
  F(NewTargetExpression, "NewTargetExpression")                               \
  F(ObjectAssignmentTarget, "ObjectAssignmentTarget")                         \
  F(ObjectBinding, "ObjectBinding")                                           \
  F(ObjectExpression, "ObjectExpression")                                     \
  F(ReturnStatement, "ReturnStatement")                                       \
  F(Script, "Script")                                                         \
  F(SetterContents, "SetterContents")                                         \
  F(ShorthandProperty, "ShorthandProperty")                                   \
  F(SpreadElement, "SpreadElement")                                           \
  F(StaticMemberAssignmentTarget, "StaticMemberAssignmentTarget")             \
  F(StaticMemberExpression, "StaticMemberExpression")                         \
  F(Super, "Super")                                                           \
  F(SwitchCase, "SwitchCase")                                                 \
  F(SwitchDefault, "SwitchDefault")                                           \
  F(SwitchStatement, "SwitchStatement")                                       \
  F(SwitchStatementWithDefault, "SwitchStatementWithDefault")                 \
  F(TemplateElement, "TemplateElement")                                       \
  F(TemplateExpression, "TemplateExpression")                                 \
  F(ThisExpression, "ThisExpression")                                         \
  F(ThrowStatement, "ThrowStatement")                                         \
  F(TryCatchStatement, "TryCatchStatement")                                   \
  F(TryFinallyStatement, "TryFinallyStatement")                               \
  F(UnaryExpression, "UnaryExpression")                                       \
  F(UpdateExpression, "UpdateExpression")                                     \
  F(VariableDeclaration, "VariableDeclaration")                               \
  F(VariableDeclarator, "VariableDeclarator")                                 \
  F(WhileStatement, "WhileStatement")                                         \
  F(WithStatement, "WithStatement")                                           \
  F(YieldExpression, "YieldExpression")                                       \
  F(YieldStarExpression, "YieldStarExpression")

enum class BinKind {
#define EMIT_ENUM(name, _) name,
  FOR_EACH_BIN_KIND(EMIT_ENUM)
#undef EMIT_ENUM
};

// The number of distinct values of BinKind.
const size_t BINKIND_LIMIT = 120;

/**
 * The different fields of Binary AST nodes, as per the specifications of
 * Binary AST.
 *
 * Usage:
 *
 * ```c++
 * #define WITH_FIELD(CPP_NAME, SPEC_NAME) ...
 * FOR_EACH_BIN_FIELD(WITH_FIELD)
 * ```
 *
 * (sorted by alphabetical order)
 */
#define FOR_EACH_BIN_FIELD(F)                         \
  F(Alternate, "alternate")                           \
  F(Arguments, "arguments")                           \
  F(Binding, "binding")                               \
  F(BindingScope, "bindingScope")                     \
  F(Body, "body")                                     \
  F(BodyScope, "bodyScope")                           \
  F(BoundNames, "boundNames")                         \
  F(Callee, "callee")                                 \
  F(Cases, "cases")                                   \
  F(CatchClause, "catchClause")                       \
  F(Consequent, "consequent")                         \
  F(Contents, "contents")                             \
  F(ContentsSkip, "contents_skip")                    \
  F(Declaration, "declaration")                       \
  F(Declarators, "declarators")                       \
  F(DeclaredNames, "declaredNames")                   \
  F(DefaultBinding, "defaultBinding")                 \
  F(DefaultCase, "defaultCase")                       \
  F(Directives, "directives")                         \
  F(Discriminant, "discriminant")                     \
  F(Elements, "elements")                             \
  F(ExportedName, "exportedName")                     \
  F(Expression, "expression")                         \
  F(Finalizer, "finalizer")                           \
  F(Flags, "flags")                                   \
  F(HasDirectEval, "hasDirectEval")                   \
  F(Index, "index")                                   \
  F(Init, "init")                                     \
  F(IsAsync, "isAsync")                               \
  F(IsCaptured, "isCaptured")                         \
  F(IsFunctionNameCaptured, "isFunctionNameCaptured") \
  F(IsGenerator, "isGenerator")                       \
  F(IsPrefix, "isPrefix")                             \
  F(IsSimpleParameterList, "isSimpleParameterList")   \
  F(IsStatic, "isStatic")                             \
  F(IsThisCaptured, "isThisCaptured")                 \
  F(Items, "items")                                   \
  F(Kind, "kind")                                     \
  F(Label, "label")                                   \
  F(Left, "left")                                     \
  F(Length, "length")                                 \
  F(Method, "method")                                 \
  F(ModuleSpecifier, "moduleSpecifier")               \
  F(Name, "name")                                     \
  F(NamedExports, "namedExports")                     \
  F(NamedImports, "namedImports")                     \
  F(NamespaceBinding, "namespaceBinding")             \
  F(Object, "object")                                 \
  F(Operand, "operand")                               \
  F(Operator, "operator")                             \
  F(Param, "param")                                   \
  F(ParamNames, "paramNames")                         \
  F(ParameterScope, "parameterScope")                 \
  F(Params, "params")                                 \
  F(Pattern, "pattern")                               \
  F(PostDefaultCases, "postDefaultCases")             \
  F(PreDefaultCases, "preDefaultCases")               \
  F(Properties, "properties")                         \
  F(Property, "property")                             \
  F(RawValue, "rawValue")                             \
  F(Rest, "rest")                                     \
  F(Right, "right")                                   \
  F(Scope, "scope")                                   \
  F(Statements, "statements")                         \
  F(Super, "super")                                   \
  F(Tag, "tag")                                       \
  F(Test, "test")                                     \
  F(Update, "update")                                 \
  F(Value, "value")

enum class BinField {
#define EMIT_ENUM(name, _) name,
  FOR_EACH_BIN_FIELD(EMIT_ENUM)
#undef EMIT_ENUM
};

// The number of distinct values of BinField.
const size_t BINFIELD_LIMIT = 69;

/**
 * The different variants of Binary AST string enums, as per
 * the specifications of Binary AST, as a single macro and
 * `enum class`.
 *
 * Separate enum classes are also defined in BinASTParser.h.
 *
 * Usage:
 *
 * ```c++
 * #define WITH_VARIANT(CPP_NAME, SPEC_NAME) ...
 * FOR_EACH_BIN_VARIANT(WITH_VARIANT)
 * ```
 *
 * (sorted by alphabetical order)
 */
#define FOR_EACH_BIN_VARIANT(F)                               \
  F(AssertedDeclaredKindConstLexical, "const lexical")        \
  F(AssertedDeclaredKindNonConstLexical, "non-const lexical") \
  F(AssertedDeclaredKindOrVariableDeclarationKindVar, "var")  \
  F(BinaryOperatorBitAnd, "&")                                \
  F(BinaryOperatorBitOr, "|")                                 \
  F(BinaryOperatorBitXor, "^")                                \
  F(BinaryOperatorComma, ",")                                 \
  F(BinaryOperatorDiv, "/")                                   \
  F(BinaryOperatorEq, "==")                                   \
  F(BinaryOperatorGeqThan, ">=")                              \
  F(BinaryOperatorGreaterThan, ">")                           \
  F(BinaryOperatorIn, "in")                                   \
  F(BinaryOperatorInstanceof, "instanceof")                   \
  F(BinaryOperatorLeqThan, "<=")                              \
  F(BinaryOperatorLessThan, "<")                              \
  F(BinaryOperatorLogicalAnd, "&&")                           \
  F(BinaryOperatorLogicalOr, "||")                            \
  F(BinaryOperatorLsh, "<<")                                  \
  F(BinaryOperatorMod, "%")                                   \
  F(BinaryOperatorMul, "*")                                   \
  F(BinaryOperatorNeq, "!=")                                  \
  F(BinaryOperatorOrUnaryOperatorMinus, "-")                  \
  F(BinaryOperatorOrUnaryOperatorPlus, "+")                   \
  F(BinaryOperatorPow, "**")                                  \
  F(BinaryOperatorRsh, ">>")                                  \
  F(BinaryOperatorStrictEq, "===")                            \
  F(BinaryOperatorStrictNeq, "!==")                           \
  F(BinaryOperatorUrsh, ">>>")                                \
  F(CompoundAssignmentOperatorBitAndAssign, "&=")             \
  F(CompoundAssignmentOperatorBitOrAssign, "|=")              \
  F(CompoundAssignmentOperatorBitXorAssign, "^=")             \
  F(CompoundAssignmentOperatorDivAssign, "/=")                \
  F(CompoundAssignmentOperatorLshAssign, "<<=")               \
  F(CompoundAssignmentOperatorMinusAssign, "-=")              \
  F(CompoundAssignmentOperatorModAssign, "%=")                \
  F(CompoundAssignmentOperatorMulAssign, "*=")                \
  F(CompoundAssignmentOperatorPlusAssign, "+=")               \
  F(CompoundAssignmentOperatorPowAssign, "**=")               \
  F(CompoundAssignmentOperatorRshAssign, ">>=")               \
  F(CompoundAssignmentOperatorUrshAssign, ">>>=")             \
  F(UnaryOperatorBitNot, "~")                                 \
  F(UnaryOperatorDelete, "delete")                            \
  F(UnaryOperatorNot, "!")                                    \
  F(UnaryOperatorTypeof, "typeof")                            \
  F(UnaryOperatorVoid, "void")                                \
  F(UpdateOperatorDecr, "--")                                 \
  F(UpdateOperatorIncr, "++")                                 \
  F(VariableDeclarationKindConst, "const")                    \
  F(VariableDeclarationKindLet, "let")

enum class BinVariant {
#define EMIT_ENUM(name, _) name,
  FOR_EACH_BIN_VARIANT(EMIT_ENUM)
#undef EMIT_ENUM
};

// The number of distinct values of BinVariant.
const size_t BINVARIANT_LIMIT = 49;

/**
 * Return a string describing a `BinKind`.
 */
const char* describeBinKind(const BinKind& kind);

/**
 * Return a string describing a `BinField`.
 */
const char* describeBinField(const BinField& kind);

/**
 * Return a string describing a `BinVariant`.
 */
const char* describeBinVariant(const BinVariant& kind);

}  // namespace frontend
}  // namespace js

#endif  // frontend_BinToken_h

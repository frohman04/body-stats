name := "body-graphs"

version := "0.1"

scalaVersion := "2.12.6"

val slf4jVersion = "1.7.25"
val plotlyVersion = "0.4.2"

libraryDependencies ++= Seq(
  "org.slf4j" % "slf4j-api" % slf4jVersion,
  "org.slf4j" % "log4j-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jcl-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jul-to-slf4j" % slf4jVersion,
  "ch.qos.logback" % "logback-classic" % "1.2.3" % "runtime",
  "com.typesafe.scala-logging" %% "scala-logging" % "3.9.0",

  "com.github.scopt" %% "scopt" % "3.7.0",
  "org.apache.commons" % "commons-math3" % "3.6.1",
  "org.apache.poi" % "poi-ooxml" % "4.0.0",
  "org.plotly-scala" % "plotly-core_2.12" % plotlyVersion,
  "org.plotly-scala" % "plotly-render_2.12" % plotlyVersion
)
